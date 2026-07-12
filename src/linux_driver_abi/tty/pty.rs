//! linux-parity: partial
//! linux-source: vendor/linux/drivers/tty/pty.c
//! test-origin: linux:vendor/linux/drivers/tty/pty.c
//! UNIX98 pseudo-terminals — `/dev/ptmx` + `/dev/pts/N`.
//!
//! Implements the master/slave pty pair that `openpt(3)` / `grantpt(3)` /
//! `unlockpt(3)` / `ptsname(3)` drive, so terminal emulators (xterm) can run a
//! shell.  Opening `/dev/ptmx` allocates a fresh pair and materialises the
//! matching `/dev/pts/N` slave node (devpts); the master fd shuttles bytes
//! through the n_tty line discipline to the slave and back.
//!
//! References:
//!   - `vendor/linux/drivers/tty/pty.c`            — pty master/slave drivers
//!   - `vendor/linux/drivers/tty/pty.c::pty_unix98_ioctl` — TIOCGPTN/TIOCSPTLCK
//!   - `vendor/linux/drivers/tty/n_tty.c`          — input/output processing
//!   - `vendor/linux/fs/devpts/inode.c`            — the /dev/pts slave nodes

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use super::{
    KernelTermios, KernelTermios2, TCFLSH, TCGETS, TCGETS2, TCSBRK, TCSETS, TCSETS2, TCSETSF,
    TCSETSF2, TCSETSW, TCSETSW2, TIOCEXCL, TIOCGPGRP, TIOCGSID, TIOCGWINSZ, TIOCNXCL, TIOCSCTTY,
    TIOCSPGRP, TIOCSWINSZ, Winsize,
};
use crate::fs::ops::{FileOps, IoctlFn, PollFn};
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EAGAIN, EFAULT, EINVAL, EIO, ENODEV, ENOTTY};
use crate::kernel::sched::wait::WaitQueueHead;
use crate::kernel::task::task_state::TASK_INTERRUPTIBLE;

/// Internal restart errno returned by Linux `n_tty_wait_for_input()` when an
/// unblocked signal interrupts a blocking read.  Syscall exit turns this into
/// either a restarted `read(2)` or userspace `EINTR`, according to `SA_RESTART`.
const ERESTARTSYS: i32 = 512;

// ── UNIX98 pty ioctls — `include/uapi/asm-generic/ioctls.h` ───────────────────
/// `TIOCGPTN` — get the slave pty number (`_IOR('T', 0x30, unsigned int)`).
pub const TIOCGPTN: u32 = 0x8004_5430;
/// `TIOCSPTLCK` — lock/unlock the slave (`_IOW('T', 0x31, int)`).
pub const TIOCSPTLCK: u32 = 0x4004_5431;
/// `TIOCGPTLCK` — read the slave lock state (`_IOR('T', 0x39, int)`).
pub const TIOCGPTLCK: u32 = 0x8004_5439;
/// `TIOCPKT` — enable/disable packet mode (`_IOW('T', 0x20, int)`).
pub const TIOCPKT: u32 = 0x5420;
/// `FIONREAD` / `TIOCINQ` — bytes available for reading.
pub const FIONREAD: u32 = 0x541B;

// ── termios flag bits we honour (asm-generic/termbits.h) ──────────────────────
const IGNCR: u32 = 0x0080;
const ICRNL: u32 = 0x0100;
const INLCR: u32 = 0x0040;
const OPOST: u32 = 0x0001;
const ONLCR: u32 = 0x0004;
const ISIG: u32 = 0x0001;
const ICANON: u32 = 0x0002;
const ECHO: u32 = 0x0008;
const ECHOE: u32 = 0x0010;
const NOFLSH: u32 = 0x0080;

// c_cc indexes (asm-generic/termbits.h).
const VINTR: usize = 0;
const VQUIT: usize = 1;
const VERASE: usize = 2;
const VKILL: usize = 3;
const VEOF: usize = 4;
const VSUSP: usize = 10;

// Signals raised by the line discipline.
const SIGHUP: i32 = 1;
const SIGINT: i32 = 2;
const SIGQUIT: i32 = 3;
const SIGTSTP: i32 = 20;

/// Linux UNIX98 pty slave major (`drivers/tty/pty.c` — `UNIX98_PTY_SLAVE_MAJOR`).
pub const UNIX98_PTY_SLAVE_MAJOR: u32 = 136;

/// A single UNIX98 pty pair.  The two byte FIFOs are the master and slave read
/// queues; `to_slave` carries the master's writes (after input processing) and
/// `to_master` carries the slave's writes and input echoes (after output
/// processing).
pub struct Pty {
    pub index: u32,
    locked: AtomicBool,
    packet: AtomicBool,
    master_open: AtomicBool,
    slave_open_count: AtomicUsize,
    /// Sticky: set once any slave fd has attached.  The master only reports
    /// hang-up/`EIO` after the slave has been opened and then fully closed, so a
    /// master that reads before its slave is spawned blocks instead of seeing a
    /// spurious EOF.
    slave_ever_opened: AtomicBool,
    termios: Mutex<KernelTermios>,
    winsize: Mutex<Winsize>,
    /// Readable from the slave (produced by master writes).
    to_slave: Mutex<VecDeque<u8>>,
    /// Readable from the master (produced by slave writes + input echo).
    to_master: Mutex<VecDeque<u8>>,
    /// Canonical-mode line-assembly buffer for the slave input side.
    canon: Mutex<Vec<u8>>,
    /// Linux `tty_struct::{read_wait,write_wait}` for the master endpoint.
    master_read_wait: WaitQueueHead,
    master_write_wait: WaitQueueHead,
    /// Linux `tty_struct::{read_wait,write_wait}` for the slave endpoint.
    slave_read_wait: WaitQueueHead,
    slave_write_wait: WaitQueueHead,
    pgrp: AtomicI32,
    session: AtomicI32,
}

impl Pty {
    fn new(index: u32) -> Arc<Self> {
        Arc::new(Self {
            index,
            // Linux allocates a devpts slave locked; unlockpt() clears it.
            locked: AtomicBool::new(true),
            packet: AtomicBool::new(false),
            master_open: AtomicBool::new(true),
            slave_open_count: AtomicUsize::new(0),
            slave_ever_opened: AtomicBool::new(false),
            termios: Mutex::new(KernelTermios::default()),
            winsize: Mutex::new(Winsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            }),
            to_slave: Mutex::new(VecDeque::new()),
            to_master: Mutex::new(VecDeque::new()),
            canon: Mutex::new(Vec::new()),
            master_read_wait: WaitQueueHead::new(),
            master_write_wait: WaitQueueHead::new(),
            slave_read_wait: WaitQueueHead::new(),
            slave_write_wait: WaitQueueHead::new(),
            pgrp: AtomicI32::new(0),
            session: AtomicI32::new(0),
        })
    }

    /// Master → slave: run the bytes through the n_tty input processing stage
    /// (CR/NL translation, signal chars, canonical line editing) and echo.
    pub fn master_write(&self, data: &[u8]) {
        let termios = *self.termios.lock();
        let icanon = termios.c_lflag & ICANON != 0;
        let echo = termios.c_lflag & ECHO != 0;
        let echoe = termios.c_lflag & ECHOE != 0;
        let isig = termios.c_lflag & ISIG != 0;
        let noflsh = termios.c_lflag & NOFLSH != 0;
        let inlcr = termios.c_iflag & INLCR != 0;
        let igncr = termios.c_iflag & IGNCR != 0;
        let icrnl = termios.c_iflag & ICRNL != 0;
        let cc = termios.c_cc;

        let mut echo_out: Vec<u8> = Vec::new();
        {
            let mut to_slave = self.to_slave.lock();
            let mut canon = self.canon.lock();
            for &b0 in data {
                let mut b = b0;

                // Input CR/NL translation (asm-generic n_tty input map).
                if b == b'\r' {
                    if igncr {
                        continue;
                    }
                    if icrnl {
                        b = b'\n';
                    }
                } else if b == b'\n' && inlcr {
                    b = b'\r';
                }

                // Signal-generating characters take precedence over queueing.
                if isig {
                    if b == cc[VINTR] {
                        self.raise_signal(SIGINT, noflsh, &mut canon, &mut to_slave);
                        continue;
                    }
                    if b == cc[VQUIT] {
                        self.raise_signal(SIGQUIT, noflsh, &mut canon, &mut to_slave);
                        continue;
                    }
                    if b == cc[VSUSP] {
                        self.raise_signal(SIGTSTP, noflsh, &mut canon, &mut to_slave);
                        continue;
                    }
                }

                if icanon {
                    if b == cc[VERASE] || b == 0x08 {
                        if canon.pop().is_some() && echo {
                            if echoe {
                                echo_out.extend_from_slice(b"\x08 \x08");
                            } else {
                                echo_out.push(b);
                            }
                        }
                    } else if b == cc[VKILL] {
                        let n = canon.len();
                        canon.clear();
                        if echo && echoe {
                            for _ in 0..n {
                                echo_out.extend_from_slice(b"\x08 \x08");
                            }
                        }
                    } else if b == cc[VEOF] {
                        // ^D flushes the pending line (possibly empty → EOF).
                        for c in canon.drain(..) {
                            to_slave.push_back(c);
                        }
                    } else if b == b'\n' {
                        canon.push(b'\n');
                        for c in canon.drain(..) {
                            to_slave.push_back(c);
                        }
                        if echo {
                            echo_out.push(b'\n');
                        }
                    } else {
                        canon.push(b);
                        if echo {
                            echo_out.push(b);
                        }
                    }
                } else {
                    to_slave.push_back(b);
                    if echo {
                        echo_out.push(b);
                    }
                }
            }
        }

        if !echo_out.is_empty() {
            self.output_to_master(&echo_out);
        }
        if self.slave_readable() {
            // `n_tty_receive_buf_common()` publishes committed input and wakes
            // `tty->read_wait` after the line discipline makes it readable.
            self.slave_read_wait.wake_up_all();
        }
    }

    fn raise_signal(
        &self,
        sig: i32,
        noflsh: bool,
        canon: &mut Vec<u8>,
        to_slave: &mut VecDeque<u8>,
    ) {
        if !noflsh {
            canon.clear();
            to_slave.clear();
        }
        let pgrp = self.pgrp.load(Ordering::Acquire);
        if pgrp > 0 {
            crate::kernel::signal::send_signal_to_process_group(pgrp, sig);
        }
    }

    /// Slave → master: apply OPOST/ONLCR output processing and queue for the
    /// master reader.
    pub fn slave_write(&self, data: &[u8]) {
        self.output_to_master(data);
    }

    fn output_to_master(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let termios = *self.termios.lock();
        let opost = termios.c_oflag & OPOST != 0;
        let onlcr = termios.c_oflag & ONLCR != 0;
        {
            let mut q = self.to_master.lock();
            for &b in data {
                if opost && onlcr && b == b'\n' {
                    q.push_back(b'\r');
                    q.push_back(b'\n');
                } else {
                    q.push_back(b);
                }
            }
        }
        self.master_read_wait.wake_up_all();
    }

    fn master_read(&self, buf: &mut [u8]) -> usize {
        drain_into(&mut self.to_master.lock(), buf)
    }

    fn slave_read(&self, buf: &mut [u8]) -> usize {
        drain_into(&mut self.to_slave.lock(), buf)
    }

    fn master_readable(&self) -> bool {
        !self.to_master.lock().is_empty()
    }

    fn slave_readable(&self) -> bool {
        !self.to_slave.lock().is_empty()
    }
}

fn drain_into(q: &mut VecDeque<u8>, buf: &mut [u8]) -> usize {
    let n = core::cmp::min(buf.len(), q.len());
    for slot in buf.iter_mut().take(n) {
        *slot = q.pop_front().unwrap();
    }
    n
}

// ── Global registry ───────────────────────────────────────────────────────────

lazy_static! {
    static ref PTYS: Mutex<BTreeMap<u32, Arc<Pty>>> = Mutex::new(BTreeMap::new());
}
static NEXT_HINT: AtomicU32 = AtomicU32::new(0);

/// Allocate a fresh pty pair and materialise its `/dev/pts/N` slave node.
fn pty_alloc() -> Arc<Pty> {
    let mut reg = PTYS.lock();
    // Lowest free index (Linux devpts uses an ida — a compact index space).
    let mut index = NEXT_HINT.load(Ordering::Relaxed);
    while reg.contains_key(&index) {
        index += 1;
    }
    let pty = Pty::new(index);
    reg.insert(index, pty.clone());
    NEXT_HINT.store(index + 1, Ordering::Relaxed);
    drop(reg);
    let _ = crate::init::rootfs::devpts_create_slave(index);
    pty
}

fn pty_lookup(index: u32) -> Option<Arc<Pty>> {
    PTYS.lock().get(&index).cloned()
}

fn pty_free(index: u32) {
    PTYS.lock().remove(&index);
    crate::init::rootfs::devpts_remove_slave(index);
    // Reuse freed slots first so the pts namespace stays compact.
    let _ = NEXT_HINT.fetch_min(index, Ordering::Relaxed);
}

#[cfg(test)]
pub fn reset_for_tests() {
    PTYS.lock().clear();
    NEXT_HINT.store(0, Ordering::Relaxed);
}

// ── File ↔ pty association ─────────────────────────────────────────────────────

/// The master pty for this `/dev/ptmx` handle.  Allocation is lazy: the first
/// operation on a freshly opened master (always an unlockpt/ptsname ioctl in
/// practice) creates the pair and stashes `index + 1` in `file.private`.
fn master_pty(file: &FileRef) -> Arc<Pty> {
    let mut slot = file.private.lock();
    if *slot != 0 {
        if let Some(pty) = pty_lookup((*slot - 1) as u32) {
            return pty;
        }
    }
    let pty = pty_alloc();
    *slot = (pty.index + 1) as usize;
    pty
}

/// The slave pty for a `/dev/pts/N` handle, identified by the numeric dentry
/// name.  There is no VFS `open` hook, so each slave `File` "attaches" on its
/// first operation: it stamps `file.private` and bumps `slave_open_count`
/// exactly once.  `dup(2)`/`fork(2)` share one `File`, so a pty opened once and
/// inherited by a shell is counted once and its `release` decrements once —
/// giving the master an accurate "last slave closed" (`EIO`) edge.
fn slave_attach(file: &FileRef) -> Option<Arc<Pty>> {
    let index: u32 = file.dentry.name.parse().ok()?;
    let pty = pty_lookup(index)?;
    let mut slot = file.private.lock();
    if *slot == 0 {
        *slot = (index + 1) as usize;
        pty.slave_open_count.fetch_add(1, Ordering::AcqRel);
        pty.slave_ever_opened.store(true, Ordering::Release);
    }
    Some(pty)
}

/// Look up a slave's pty without attaching (for `release`, which must not
/// resurrect a count).
fn slave_lookup(file: &FileRef) -> Option<Arc<Pty>> {
    let index: u32 = file.dentry.name.parse().ok()?;
    pty_lookup(index)
}

#[cfg(not(test))]
fn pty_yield() {
    unsafe {
        crate::kernel::sched::schedule_with_irqs_enabled();
    }
}

#[cfg(test)]
fn pty_yield() {}

fn task_has_unblocked_signal(task: *mut crate::kernel::task::TaskStruct) -> bool {
    !task.is_null() && crate::kernel::signal::has_unblocked_pending_signals(task)
}

fn is_nonblock(file: &FileRef) -> bool {
    file.flags.load(Ordering::Acquire) & crate::include::uapi::fcntl::O_NONBLOCK != 0
}

// ── Master (`/dev/ptmx`) file operations ───────────────────────────────────────

fn ptmx_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    if buf.is_empty() {
        return Ok(0);
    }
    let pty = master_pty(file);
    loop {
        let n = pty.master_read(buf);
        if n > 0 {
            return Ok(n);
        }
        // `pty_close(slave)` sets `TTY_OTHER_CLOSED` on the master;
        // `n_tty_wait_for_input()` reports `-EIO` once buffered data is gone.
        if pty.slave_ever_opened.load(Ordering::Acquire)
            && pty.slave_open_count.load(Ordering::Acquire) == 0
        {
            return Err(EIO);
        }
        if is_nonblock(file) {
            return Err(EAGAIN);
        }

        let current = unsafe { crate::kernel::sched::get_current() };
        if current.is_null() {
            pty_yield();
            continue;
        }
        unsafe {
            pty.master_read_wait
                .prepare_to_wait(current, TASK_INTERRUPTIBLE);
        }
        // Linux installs the wait entry before its final availability/hangup
        // test, closing the producer-wakeup versus schedule race.
        if pty.master_readable()
            || (pty.slave_ever_opened.load(Ordering::Acquire)
                && pty.slave_open_count.load(Ordering::Acquire) == 0)
        {
            unsafe {
                pty.master_read_wait.finish_wait(current);
            }
            continue;
        }
        if task_has_unblocked_signal(current) {
            unsafe {
                pty.master_read_wait.finish_wait(current);
            }
            return Err(ERESTARTSYS);
        }
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
            pty.master_read_wait.finish_wait(current);
        }
    }
}

fn ptmx_write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let pty = master_pty(file);
    if pty.slave_ever_opened.load(Ordering::Acquire)
        && pty.slave_open_count.load(Ordering::Acquire) == 0
    {
        return Err(EIO);
    }
    pty.master_write(buf);
    *pos = pos.saturating_add(buf.len() as u64);
    Ok(buf.len())
}

fn ptmx_poll(file: &FileRef, mut table: Option<&mut crate::fs::select::PollTable>) -> u32 {
    use crate::fs::eventpoll::{EPOLLHUP, EPOLLIN, EPOLLOUT, EPOLLRDNORM, EPOLLWRNORM};
    let pty = master_pty(file);
    // `n_tty_poll()` registers both queues before sampling state.
    crate::fs::select::poll_wait(file, &pty.master_read_wait, table.as_deref_mut());
    crate::fs::select::poll_wait(file, &pty.master_write_wait, table.as_deref_mut());
    let mut mask = EPOLLOUT | EPOLLWRNORM;
    if pty.master_readable() {
        mask |= EPOLLIN | EPOLLRDNORM;
    }
    if pty.slave_ever_opened.load(Ordering::Acquire)
        && pty.slave_open_count.load(Ordering::Acquire) == 0
        && !pty.master_readable()
    {
        mask |= EPOLLHUP;
    }
    mask
}

fn ptmx_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let pty = master_pty(file);
    match cmd {
        TIOCGPTN => {
            put_user_u32(arg, pty.index)?;
            Ok(0)
        }
        TIOCSPTLCK => {
            let v = get_user_i32(arg)?;
            pty.locked.store(v != 0, Ordering::Release);
            Ok(0)
        }
        TIOCGPTLCK => {
            put_user_u32(arg, pty.locked.load(Ordering::Acquire) as u32)?;
            Ok(0)
        }
        TIOCPKT => {
            let v = get_user_i32(arg)?;
            pty.packet.store(v != 0, Ordering::Release);
            Ok(0)
        }
        // On the master, TIOCINQ/FIONREAD reports bytes readable *from* the
        // master (i.e. the slave's pending output).
        FIONREAD => {
            put_user_u32(arg, pty.to_master.lock().len() as u32)?;
            Ok(0)
        }
        _ => pty_common_ioctl(&pty, cmd, arg),
    }
}

fn ptmx_release(file: FileRef) {
    let index = {
        let slot = file.private.lock();
        if *slot == 0 {
            return;
        }
        (*slot - 1) as u32
    };
    if let Some(pty) = pty_lookup(index) {
        pty.master_open.store(false, Ordering::Release);
        // Linux `pty_close()` wakes read/write waiters on both linked ttys so
        // blocked I/O and pollers can observe `TTY_OTHER_CLOSED`/hangup.
        pty.master_read_wait.wake_up_all();
        pty.master_write_wait.wake_up_all();
        pty.slave_read_wait.wake_up_all();
        pty.slave_write_wait.wake_up_all();
        // Closing the master hangs up the slave session.
        let pgrp = pty.pgrp.load(Ordering::Acquire);
        if pgrp > 0 {
            crate::kernel::signal::send_signal_to_process_group(pgrp, SIGHUP);
        }
        // Keep the linked tty object alive while an already-open slave file
        // (including a poll-table file pin) still refers to its wait queues.
        // The devpts node is removed now, as Linux does in the master close.
        crate::init::rootfs::devpts_remove_slave(index);
        if pty.slave_open_count.load(Ordering::Acquire) == 0 {
            pty_free(index);
        }
    }
}

// ── Slave (`/dev/pts/N`) file operations ───────────────────────────────────────

fn pts_read(file: &FileRef, buf: &mut [u8], _pos: &mut u64) -> Result<usize, i32> {
    if buf.is_empty() {
        return Ok(0);
    }
    let pty = slave_attach(file).ok_or(EIO)?;
    loop {
        let n = pty.slave_read(buf);
        if n > 0 {
            return Ok(n);
        }
        // Master gone → EOF for the slave.
        if !pty.master_open.load(Ordering::Acquire) {
            return Ok(0);
        }
        if is_nonblock(file) {
            return Err(EAGAIN);
        }

        let current = unsafe { crate::kernel::sched::get_current() };
        if current.is_null() {
            pty_yield();
            continue;
        }
        unsafe {
            pty.slave_read_wait
                .prepare_to_wait(current, TASK_INTERRUPTIBLE);
        }
        if pty.slave_readable() || !pty.master_open.load(Ordering::Acquire) {
            unsafe {
                pty.slave_read_wait.finish_wait(current);
            }
            continue;
        }
        if task_has_unblocked_signal(current) {
            unsafe {
                pty.slave_read_wait.finish_wait(current);
            }
            return Err(ERESTARTSYS);
        }
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
            pty.slave_read_wait.finish_wait(current);
        }
    }
}

fn pts_write(file: &FileRef, buf: &[u8], pos: &mut u64) -> Result<usize, i32> {
    let pty = slave_attach(file).ok_or(EIO)?;
    if !pty.master_open.load(Ordering::Acquire) {
        return Err(EIO);
    }
    pty.slave_write(buf);
    *pos = pos.saturating_add(buf.len() as u64);
    Ok(buf.len())
}

fn pts_poll(file: &FileRef, mut table: Option<&mut crate::fs::select::PollTable>) -> u32 {
    use crate::fs::eventpoll::{EPOLLHUP, EPOLLIN, EPOLLOUT, EPOLLRDNORM, EPOLLWRNORM};
    let Some(pty) = slave_attach(file) else {
        return EPOLLHUP;
    };
    crate::fs::select::poll_wait(file, &pty.slave_read_wait, table.as_deref_mut());
    crate::fs::select::poll_wait(file, &pty.slave_write_wait, table.as_deref_mut());
    let mut mask = EPOLLOUT | EPOLLWRNORM;
    if pty.slave_readable() {
        mask |= EPOLLIN | EPOLLRDNORM;
    }
    if !pty.master_open.load(Ordering::Acquire) {
        mask |= EPOLLHUP;
    }
    mask
}

fn pts_ioctl(file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    let pty = slave_attach(file).ok_or(ENODEV)?;
    pty_common_ioctl(&pty, cmd, arg)
}

fn pts_release(file: FileRef) {
    // Only decrement if this File actually attached (stamped `file.private`).
    if *file.private.lock() == 0 {
        return;
    }
    if let Some(pty) = slave_lookup(&file) {
        let prev = pty
            .slave_open_count
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                count.checked_sub(1)
            })
            .unwrap_or(0);
        pty.master_read_wait.wake_up_all();
        pty.master_write_wait.wake_up_all();
        pty.slave_read_wait.wake_up_all();
        pty.slave_write_wait.wake_up_all();
        if prev == 1 && !pty.master_open.load(Ordering::Acquire) {
            pty_free(pty.index);
        }
    }
}

// ── Shared job-control / termios ioctls ────────────────────────────────────────

fn pty_common_ioctl(pty: &Arc<Pty>, cmd: u32, arg: u64) -> Result<i64, i32> {
    match cmd {
        TCGETS => {
            let t = *pty.termios.lock();
            copy_struct_to_user(arg, &t)?;
            Ok(0)
        }
        TCSETS | TCSETSW | TCSETSF => {
            let t: KernelTermios = copy_struct_from_user(arg)?;
            *pty.termios.lock() = t;
            Ok(0)
        }
        TCGETS2 => {
            let t2 = KernelTermios2::from(*pty.termios.lock());
            copy_struct_to_user(arg, &t2)?;
            Ok(0)
        }
        TCSETS2 | TCSETSW2 | TCSETSF2 => {
            let t2: KernelTermios2 = copy_struct_from_user(arg)?;
            *pty.termios.lock() = kernel_termios_from2(t2);
            Ok(0)
        }
        TIOCGWINSZ => {
            let ws = *pty.winsize.lock();
            copy_struct_to_user(arg, &ws)?;
            Ok(0)
        }
        TIOCSWINSZ => {
            let ws: Winsize = copy_struct_from_user(arg)?;
            *pty.winsize.lock() = ws;
            // Linux notifies the foreground group of the resize.
            let pgrp = pty.pgrp.load(Ordering::Acquire);
            if pgrp > 0 {
                const SIGWINCH: i32 = 28;
                crate::kernel::signal::send_signal_to_process_group(pgrp, SIGWINCH);
            }
            Ok(0)
        }
        TIOCSCTTY => {
            let (sid, pgrp) = current_session_and_pgrp().unwrap_or((0, 0));
            pty.session.store(sid, Ordering::Release);
            if pgrp != 0 && pty.pgrp.load(Ordering::Acquire) == 0 {
                pty.pgrp.store(pgrp, Ordering::Release);
            }
            Ok(0)
        }
        TIOCGPGRP => {
            put_user_u32(arg, pty.pgrp.load(Ordering::Acquire) as u32)?;
            Ok(0)
        }
        TIOCSPGRP => {
            let pgrp = get_user_i32(arg)?;
            pty.pgrp.store(pgrp, Ordering::Release);
            Ok(0)
        }
        TIOCGSID => {
            put_user_u32(arg, pty.session.load(Ordering::Acquire) as u32)?;
            Ok(0)
        }
        // On the slave, TIOCINQ/FIONREAD reports bytes readable *from* the
        // slave (the master's pending input after line-discipline processing).
        FIONREAD => {
            let n = pty.to_slave.lock().len() as u32;
            put_user_u32(arg, n)?;
            Ok(0)
        }
        // Break/flush/exclusive-mode requests are accepted as no-ops, matching
        // the console tty compat path (`tty_ioctl_compat`).
        TCSBRK | TCFLSH | TIOCEXCL | TIOCNXCL => Ok(0),
        _ => Err(ENOTTY),
    }
}

fn kernel_termios_from2(t2: KernelTermios2) -> KernelTermios {
    let mut c_cc = KernelTermios::default().c_cc;
    let n = core::cmp::min(c_cc.len(), t2.c_cc.len());
    c_cc[..n].copy_from_slice(&t2.c_cc[..n]);
    KernelTermios {
        c_iflag: t2.c_iflag,
        c_oflag: t2.c_oflag,
        c_cflag: t2.c_cflag,
        c_lflag: t2.c_lflag,
        c_line: t2.c_line,
        c_cc,
        c_ispeed: t2.c_ispeed,
        c_ospeed: t2.c_ospeed,
    }
}

fn current_session_and_pgrp() -> Option<(i32, i32)> {
    let task = unsafe { crate::kernel::sched::get_current() };
    if task.is_null() {
        return None;
    }
    let pid = unsafe { (*task).pid };
    let sid = crate::kernel::session::session_id(pid).unwrap_or(pid);
    let pgrp = crate::kernel::session::process_group(pid).unwrap_or(pid);
    Some((sid, pgrp))
}

// ── userspace copy helpers ─────────────────────────────────────────────────────

fn put_user_u32(arg: u64, val: u32) -> Result<(), i32> {
    if arg == 0 {
        return Err(EFAULT);
    }
    unsafe { crate::arch::x86::kernel::uaccess::put_user_u32(arg as *mut u32, val) }.map_err(|e| -e)
}

fn get_user_i32(arg: u64) -> Result<i32, i32> {
    if arg == 0 {
        return Err(EFAULT);
    }
    let v = unsafe { crate::arch::x86::kernel::uaccess::get_user_u32(arg as *const u32) }
        .map_err(|e| -e)?;
    Ok(v as i32)
}

fn copy_struct_to_user<T>(arg: u64, val: &T) -> Result<(), i32> {
    if arg == 0 {
        return Err(EFAULT);
    }
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(
            arg as *mut u8,
            val as *const T as *const u8,
            core::mem::size_of::<T>(),
        )
    };
    if not_copied == 0 { Ok(()) } else { Err(EFAULT) }
}

fn copy_struct_from_user<T: Default>(arg: u64) -> Result<T, i32> {
    if arg == 0 {
        return Err(EFAULT);
    }
    let mut val = T::default();
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            &mut val as *mut T as *mut u8,
            arg as *const u8,
            core::mem::size_of::<T>(),
        )
    };
    if not_copied == 0 {
        Ok(val)
    } else {
        Err(EFAULT)
    }
}

// ── file_operations tables ─────────────────────────────────────────────────────

/// `/dev/ptmx` — the pty master multiplexor.
pub static PTMX_FILE_OPS: FileOps = FileOps {
    name: "ptmx",
    read: Some(ptmx_read),
    write: Some(ptmx_write),
    llseek: None,
    fsync: None,
    poll: Some(ptmx_poll as PollFn),
    ioctl: Some(ptmx_ioctl as IoctlFn),
    mmap: None,
    release: Some(ptmx_release),
    readdir: None,
};

/// `/dev/pts/N` — a pty slave.
pub static PTS_SLAVE_FILE_OPS: FileOps = FileOps {
    name: "pts",
    read: Some(pts_read),
    write: Some(pts_write),
    llseek: None,
    fsync: None,
    poll: Some(pts_poll as PollFn),
    ioctl: Some(pts_ioctl as IoctlFn),
    mmap: None,
    release: Some(pts_release),
    readdir: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_termios(pty: &Arc<Pty>) {
        let mut t = pty.termios.lock();
        t.c_lflag &= !(ICANON | ECHO | ISIG);
        t.c_oflag &= !OPOST;
    }

    #[test]
    fn canonical_line_delivered_on_newline() {
        let pty = Pty::new(0);
        pty.master_write(b"ls");
        // Nothing delivered to the slave until the line terminates.
        let mut buf = [0u8; 16];
        assert_eq!(pty.slave_read(&mut buf), 0);
        pty.master_write(b"\n");
        let n = pty.slave_read(&mut buf);
        assert_eq!(&buf[..n], b"ls\n");
    }

    #[test]
    fn canonical_echo_reaches_master() {
        let pty = Pty::new(1);
        pty.master_write(b"hi\n");
        let mut buf = [0u8; 16];
        let n = pty.master_read(&mut buf);
        // Echo with ONLCR maps the newline to CRLF.
        assert_eq!(&buf[..n], b"hi\r\n");
    }

    #[test]
    fn erase_removes_last_char() {
        let pty = Pty::new(2);
        pty.master_write(b"ax\x7f\n");
        let mut buf = [0u8; 16];
        let n = pty.slave_read(&mut buf);
        assert_eq!(&buf[..n], b"a\n");
    }

    #[test]
    fn raw_mode_passes_bytes_through_immediately() {
        let pty = Pty::new(3);
        raw_termios(&pty);
        pty.master_write(b"abc");
        let mut buf = [0u8; 16];
        let n = pty.slave_read(&mut buf);
        assert_eq!(&buf[..n], b"abc");
    }

    #[test]
    fn slave_output_gets_onlcr() {
        let pty = Pty::new(4);
        pty.slave_write(b"line\n");
        let mut buf = [0u8; 16];
        let n = pty.master_read(&mut buf);
        assert_eq!(&buf[..n], b"line\r\n");
    }

    #[test]
    fn cr_translated_to_nl_on_input() {
        let pty = Pty::new(5);
        // Enter key sends CR; ICRNL (default) maps it to NL so the line flushes.
        pty.master_write(b"cmd\r");
        let mut buf = [0u8; 16];
        let n = pty.slave_read(&mut buf);
        assert_eq!(&buf[..n], b"cmd\n");
    }
}
