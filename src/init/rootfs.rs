//! linux-parity: partial
//! linux-source: vendor/linux/init
//! test-origin: linux:vendor/linux/init
//! Initramfs root bootstrap: unpack initramfs into a ramfs root, create a
//! minimal devtmpfs layout, then mount procfs and sysfs on top.
//!
//! Mirrors the Linux early-boot flow split across:
//!   - `init/initramfs.c` - unpack rootfs image
//!   - `init/do_mounts.c` - prepare the initial root mount
//!   - `vendor/linux/init/noinitramfs.c` - fallback `/dev/console` rootfs
//!   - `vendor/linux/init/do_mounts_initrd.c` - deprecated initrd switches
//!   - `vendor/linux/init/do_mounts_rd.c` - legacy ramdisk image probing
//!   - `drivers/base/devtmpfs.c` - populate `/dev`
//!
//! Covered: rootfs/devtmpfs bootstrap, newc materialization, hardlinks, basic
//! special nodes, and disk-root switching. Deferred: decompression, streaming
//! unpack FSM, `/initrd.image` writeback, NFS/CIFS root matrices, and rdev
//! persistence in `Inode`/stat output.

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::fs::dcache::{d_alloc_child, d_lookup};
use crate::fs::file::{alloc_file, fput};
use crate::fs::libfs::{empty_ram_bytes, static_cow_bytes};
use crate::fs::mount::{self, Mount, do_mount, path_walk, set_rootfs};
use crate::fs::ops::FileOps;
use crate::fs::ramfs::{RAMFS_FILE_INODE_OPS, RAMFS_FILE_OPS, ramfs_symlink};
use crate::fs::read_write::{vfs_read, vfs_write};
use crate::fs::super_block::mount_fs;
use crate::fs::types::{Inode, InodeKind, InodePrivate, init_inode_metadata, touch_inode_now};
use crate::fs::{self, DentryRef};
use crate::include::uapi::errno::{EINVAL, ENODEV, ENOENT, ENOEXEC, ENOSPC, EOPNOTSUPP, EROFS};
use crate::include::uapi::fcntl::{O_NONBLOCK, O_RDONLY, O_RDWR};
use crate::include::uapi::mount::{MS_RDONLY, MS_REMOUNT};
use crate::init::boot::BootOptions;
use crate::init::initramfs;
use crate::init::version;
use crate::kernel::module::{self, LoadModuleError};
use lazy_static::lazy_static;
use spin::Mutex;

const DEFAULT_DIR_MODE: u32 = 0o755;
const DEFAULT_FILE_MODE: u32 = 0o644;
const INITRAMFS_CONTIG_FILE_LIMIT: usize =
    (1usize << crate::mm::zone::MAX_PAGE_ORDER) * crate::mm::frame::PAGE_SIZE;
pub const LINUX_INITRD_BLOCK_SIZE: usize = crate::init::do_mounts_rd::BLOCK_SIZE;
const CRAMFS_MAGIC_LE: [u8; 4] = crate::init::do_mounts_rd::CRAMFS_MAGIC_LE;
const DEFAULT_DISK_ROOT_FS: &str = "ext4";
#[cfg(not(test))]
const DISK_ROOT_WAIT_POLLS: usize = 200_000;
#[cfg(test)]
const DISK_ROOT_WAIT_POLLS: usize = 4;
#[cfg(not(test))]
const DISK_ROOT_WAIT_MSECS: u64 = 10_000;
const DISK_ROOT_READY_SETTLE_POLLS: usize = 128;

pub static DEV_FULL_FILE_OPS: FileOps = FileOps {
    name: "dev_full",
    read: Some(dev_full_read),
    write: Some(dev_full_write),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: None,
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

fn dev_full_read(
    _file: &crate::fs::types::FileRef,
    buf: &mut [u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    buf.fill(0);
    *pos = pos.saturating_add(buf.len() as u64);
    Ok(buf.len())
}

fn dev_full_write(
    _file: &crate::fs::types::FileRef,
    _buf: &[u8],
    _pos: &mut u64,
) -> Result<usize, i32> {
    Err(ENOSPC)
}

pub(crate) static DEV_KMSG_FILE_OPS: FileOps = FileOps {
    name: "dev_kmsg",
    read: Some(dev_kmsg_read),
    write: Some(dev_kmsg_write),
    llseek: None,
    fsync: Some(|_| Ok(())),
    poll: Some(dev_kmsg_poll),
    ioctl: None,
    mmap: None,
    release: None,
    readdir: None,
};

fn dev_kmsg_read(
    file: &crate::fs::types::FileRef,
    buf: &mut [u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    if buf.is_empty() {
        return Ok(0);
    }

    let mut cursor = file.private.lock();
    let mut seq = (*cursor as u64).max(crate::kernel::printk::PRINTK_RB.tail());
    let head = crate::kernel::printk::PRINTK_RB.head();
    if seq >= head {
        *cursor = head as usize;
        if file.flags.load(Ordering::Acquire) & O_NONBLOCK != 0 {
            return Err(crate::include::uapi::errno::EAGAIN);
        }
        return Ok(0);
    }

    let mut info = crate::kernel::printk::PrintkInfo::empty();
    let mut text = vec![0u8; crate::kernel::printk::ringbuffer::PRB_TEXT_BUF_SIZE];
    let text_len = loop {
        match crate::kernel::printk::PRINTK_RB.read(seq, &mut info, &mut text) {
            Some(n) => break n,
            None => {
                seq = crate::kernel::printk::PRINTK_RB.tail();
                if seq >= crate::kernel::printk::PRINTK_RB.head() {
                    *cursor = seq as usize;
                    if file.flags.load(Ordering::Acquire) & O_NONBLOCK != 0 {
                        return Err(crate::include::uapi::errno::EAGAIN);
                    }
                    return Ok(0);
                }
            }
        }
    };
    let line = crate::kernel::printk::render::format_dev_kmsg(&info, &text[..text_len]);
    let bytes = line.as_bytes();
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    *cursor = seq.saturating_add(1) as usize;
    *pos = pos.saturating_add(n as u64);
    Ok(n)
}

fn dev_kmsg_write(
    _file: &crate::fs::types::FileRef,
    buf: &[u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    let ts_nsec = crate::kernel::time::ktime_get();
    let _ = crate::kernel::printk::PRINTK_RB.emit(
        ts_nsec,
        crate::kernel::printk::LOG_KERN,
        crate::kernel::printk::KERN_INFO,
        0,
        0x8000_0000,
        buf,
    );
    *pos = pos.saturating_add(buf.len() as u64);
    Ok(buf.len())
}

fn dev_kmsg_poll(file: &crate::fs::types::FileRef) -> u32 {
    let cursor = *file.private.lock() as u64;
    let next = cursor.max(crate::kernel::printk::PRINTK_RB.tail());
    if next < crate::kernel::printk::PRINTK_RB.head() {
        crate::fs::proc::kmsg::EPOLLIN | crate::fs::proc::kmsg::EPOLLRDNORM
    } else {
        0
    }
}

lazy_static! {
    static ref CONSOLE_CANON_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    static ref CONSOLE_READY_BUFFER: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
    static ref DISK_ROOT_DEVICE_ALIAS: Mutex<Option<String>> = Mutex::new(None);
}

pub(crate) fn queue_console_input_response(bytes: &[u8]) {
    CONSOLE_READY_BUFFER.lock().extend(bytes.iter().copied());
}

#[cfg(test)]
lazy_static! {
    /// Simulated hardware input queue for tests. Bytes are handed back one
    /// per `try_console_input()` call, mirroring the real i8042/serial path
    /// where a multi-byte key sequence is queued together but popped a byte
    /// at a time. Lets tests exercise `console_read`'s drain-to-exhaustion
    /// behavior, which `push_console_input_for_tests` (which writes straight
    /// to the ready buffer) bypasses.
    static ref TEST_HW_INPUT_QUEUE: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
}

#[cfg(test)]
pub(crate) fn push_hardware_input_for_tests(bytes: &[u8]) {
    TEST_HW_INPUT_QUEUE.lock().extend(bytes.iter().copied());
}

#[cfg(test)]
pub(crate) fn clear_console_input_for_tests() {
    CONSOLE_CANON_BUFFER.lock().clear();
    CONSOLE_READY_BUFFER.lock().clear();
    TEST_HW_INPUT_QUEUE.lock().clear();
}

#[cfg(test)]
static CONSOLE_WAIT_TEST_INJECT_NEWLINE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static CONSOLE_WAIT_TEST_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyRamdiskImage {
    Gzip,
    Bzip2,
    Lzma,
    Xz,
    Lzo,
    Lz4,
    Romfs,
    Cramfs,
    Squashfs,
    Ext2,
    Minix,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyInitrdLoad {
    Disabled,
    NoImage,
    Unsupported {
        image: LegacyRamdiskImage,
        errno: i32,
    },
    InvalidImage {
        errno: i32,
    },
}

pub fn drain_console_control_bytes() {
    while let Some(input) = try_console_input() {
        process_console_input(input);
    }
}

fn deliver_console_signal(sig: i32) {
    // Linux n_tty `__isig()` sends terminal-generated signals to the TTY
    // foreground process group (`vendor/linux/drivers/tty/n_tty.c:1044`),
    // and silently drops the signal if `tty_get_pgrp(tty)` returns NULL.
    // We mirror that: when a foreground pgrp has been established
    // (`tty_ioctl_compat(TIOCSCTTY)` ran), route to it; otherwise fall
    // back to the current task ONLY if no controlling-tty session has
    // been claimed yet (the very early agetty / login phase).  This
    // matches Linux for normal interactive use and keeps the early-boot
    // console kill-switch alive without delivering stray signals into
    // unrelated tasks once a session exists.
    if crate::linux_driver_abi::tty::signal_compat_foreground(sig) == 0 {
        return;
    }
    if crate::linux_driver_abi::tty::compat_tty_session_active() {
        return;
    }
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() {
        let _ = unsafe { crate::kernel::signal::send_signal_to_task(current, sig) };
    }
}

/// Common Linux `n_tty_receive_signal_char` body — echo the signal char
/// glyph, optionally flush the canonical input buffer (gated by L_NOFLSH),
/// then deliver the signal to the tty foreground process group.  Ref:
/// `vendor/linux/drivers/tty/n_tty.c::isig` and `n_tty_receive_signal_char`.
fn deliver_signal_char(
    echo: bool,
    glyph: &[u8],
    sig: i32,
    termios: &crate::linux_driver_abi::tty::KernelTermios,
) {
    // The signal is processed first to alert any current readers/writers
    // to discontinue and exit their I/O loops (matches the comment in
    // Linux's `isig` body at n_tty.c:1063).  The input flush below only
    // runs when NOFLSH is not set, matching Linux's gate.
    if echo {
        echo_console_bytes(glyph);
    }
    if !crate::linux_driver_abi::tty::termios_noflsh(termios) {
        CONSOLE_CANON_BUFFER.lock().clear();
    }
    deliver_console_signal(sig);
}

#[cfg(not(test))]
fn try_console_input() -> Option<ConsoleInput> {
    crate::linux_driver_abi::input::i8042::try_read_input()
        .map(ConsoleInput::from)
        .or_else(|| crate::linux_driver_abi::tty::serial::try_read_byte().map(ConsoleInput::Byte))
}

#[cfg(test)]
fn try_console_input() -> Option<ConsoleInput> {
    TEST_HW_INPUT_QUEUE
        .lock()
        .pop_front()
        .map(ConsoleInput::Byte)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConsoleInput {
    Byte(u8),
    Shutdown,
    Restart,
}

#[cfg(not(test))]
impl From<crate::linux_driver_abi::input::i8042::ConsoleInput> for ConsoleInput {
    fn from(input: crate::linux_driver_abi::input::i8042::ConsoleInput) -> Self {
        match input {
            crate::linux_driver_abi::input::i8042::ConsoleInput::Byte(byte) => Self::Byte(byte),
            crate::linux_driver_abi::input::i8042::ConsoleInput::Shutdown => Self::Shutdown,
            crate::linux_driver_abi::input::i8042::ConsoleInput::Restart => Self::Restart,
        }
    }
}

fn echo_console_bytes(bytes: &[u8]) {
    // Serial RX FIFOs are tiny. Echo input without immediately spinning on TX
    // so `console_read()` can drain a pasted/scripted line before maintenance
    // flushes the serial queue.
    crate::kernel::console::write_bytes_deferred(bytes);
}

fn process_console_input(input: ConsoleInput) {
    match input {
        ConsoleInput::Byte(byte) => process_console_input_byte(byte),
        ConsoleInput::Shutdown => handle_login_screen_control(ShutdownControl::PowerOff),
        ConsoleInput::Restart => handle_login_screen_control(ShutdownControl::Restart),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShutdownControl {
    PowerOff,
    Restart,
}

fn handle_login_screen_control(control: ShutdownControl) {
    if !current_task_is_login_reader() {
        return;
    }
    match control {
        ShutdownControl::PowerOff => {
            echo_console_bytes(b"\nlogin control: shutting down\n");
            shutdown_from_login_control();
        }
        ShutdownControl::Restart => {
            echo_console_bytes(b"\nlogin control: restarting\n");
            restart_from_login_control();
        }
    }
}

fn current_task_is_login_reader() -> bool {
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        return false;
    }
    let comm = unsafe { &(*current).comm };
    comm_starts_with(comm, b"agetty") || comm_starts_with(comm, b"login")
}

fn comm_starts_with(comm: &[u8], name: &[u8]) -> bool {
    comm.len() >= name.len() && &comm[..name.len()] == name
}

#[cfg(feature = "qemu-test")]
fn shutdown_from_login_control() -> ! {
    crate::linux_driver_abi::platform::qemu::exit_success();
}

#[cfg(not(feature = "qemu-test"))]
fn shutdown_from_login_control() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}

fn restart_from_login_control() -> ! {
    unsafe { crate::arch::x86::kernel::reboot::reboot_via_keyboard_controller() }
}

fn process_console_input_byte(byte: u8) {
    let termios = crate::linux_driver_abi::tty::compat_termios();
    let echo = crate::linux_driver_abi::tty::termios_echo(&termios);
    let canonical = crate::linux_driver_abi::tty::termios_canonical(&termios);
    let isig = crate::linux_driver_abi::tty::termios_isig(&termios);
    // Linux n_tty matches signal chars against `tty->termios.c_cc[VINTR/
    // VQUIT/VSUSP]`, not hard-coded literals — that's how `stty intr ^A`
    // works.  Ref: vendor/linux/drivers/tty/n_tty.c::n_tty_receive_char_
    // special (the `INTR_CHAR(tty)` / `QUIT_CHAR(tty)` / `SUSP_CHAR(tty)`
    // checks at n_tty.c:1339-1346).  `_POSIX_VDISABLE` (0xff) means "no
    // mapping" — Linux ignores a `0xff` c_cc slot, and so do we.
    let intr = termios.c_cc[crate::linux_driver_abi::tty::VINTR];
    let quit = termios.c_cc[crate::linux_driver_abi::tty::VQUIT];
    let susp = termios.c_cc[crate::linux_driver_abi::tty::VSUSP];
    let posix_vdisable = 0xff_u8;

    if isig && intr != posix_vdisable && byte == intr {
        deliver_signal_char(echo, b"^C\n", crate::kernel::signal::SIGINT, &termios);
        return;
    }
    if isig && quit != posix_vdisable && byte == quit {
        // SIGQUIT (^\ by default).  Linux echoes `^\\` per
        // n_tty.c::echo_char.
        deliver_signal_char(echo, b"^\\\n", crate::kernel::signal::SIGQUIT, &termios);
        return;
    }
    if isig && susp != posix_vdisable && byte == susp {
        deliver_signal_char(echo, b"^Z\n", crate::kernel::signal::SIGTSTP, &termios);
        return;
    }

    if !canonical {
        CONSOLE_READY_BUFFER.lock().push_back(byte);
        if echo {
            echo_input_byte(byte);
        }
        return;
    }

    match byte {
        0x08 | 0x7f => {
            let erased = CONSOLE_CANON_BUFFER.lock().pop().is_some();
            if erased && echo {
                echo_console_bytes(b"\x08 \x08");
            }
        }
        b'\r' | b'\n' => {
            {
                let mut canon = CONSOLE_CANON_BUFFER.lock();
                canon.push(b'\n');
                CONSOLE_READY_BUFFER.lock().extend(canon.drain(..));
            }
            if echo {
                echo_console_bytes(b"\n");
            }
        }
        0x04 => {
            let mut canon = CONSOLE_CANON_BUFFER.lock();
            if !canon.is_empty() {
                CONSOLE_READY_BUFFER.lock().extend(canon.drain(..));
            }
        }
        byte => {
            CONSOLE_CANON_BUFFER.lock().push(byte);
            if echo {
                echo_input_byte(byte);
            }
        }
    }
}

fn echo_input_byte(byte: u8) {
    match byte {
        b'\r' | b'\n' => echo_console_bytes(b"\n"),
        0x08 | 0x7f => echo_console_bytes(b"\x08 \x08"),
        0x20..=0x7e | b'\t' => echo_console_bytes(&[byte]),
        _ => {}
    }
}

fn drain_console_ready(buf: &mut [u8]) -> usize {
    let mut ready = CONSOLE_READY_BUFFER.lock();
    let canonical = crate::linux_driver_abi::tty::termios_canonical(
        &crate::linux_driver_abi::tty::compat_termios(),
    );
    let mut n = 0usize;
    while n < buf.len() {
        let Some(byte) = ready.pop_front() else {
            break;
        };
        buf[n] = byte;
        n += 1;
        if canonical && byte == b'\n' {
            break;
        }
    }
    n
}

fn current_has_pending_signal() -> bool {
    let current = unsafe { crate::kernel::sched::get_current() };
    crate::kernel::signal::has_pending_signals(current)
}

fn console_read_ready_or_signal(buf: &mut [u8], pos: &mut u64) -> Result<Option<usize>, i32> {
    let n = drain_console_ready(buf);
    if n > 0 {
        *pos = pos.saturating_add(n as u64);
        return Ok(Some(n));
    }

    if current_has_pending_signal() {
        return Err(crate::include::uapi::errno::EINTR);
    }

    Ok(None)
}

fn console_line_edit_pending() -> bool {
    let termios = crate::linux_driver_abi::tty::compat_termios();
    crate::linux_driver_abi::tty::termios_canonical(&termios)
        && !CONSOLE_CANON_BUFFER.lock().is_empty()
}

fn reset_console_buffers() {
    CONSOLE_CANON_BUFFER.lock().clear();
    CONSOLE_READY_BUFFER.lock().clear();
    crate::linux_driver_abi::tty::reset_compat_tty_state();
    #[cfg(test)]
    {
        CONSOLE_WAIT_TEST_INJECT_NEWLINE.store(false, Ordering::SeqCst);
        CONSOLE_WAIT_TEST_COUNT.store(0, Ordering::SeqCst);
    }
}

#[cfg(not(test))]
fn console_wait_for_input(run_console_maintenance: bool) {
    if run_console_maintenance {
        crate::kernel::console::refresh_cursor_blink();
        crate::kernel::console::maintenance_budgeted();
    }
    unsafe {
        crate::kernel::sched::schedule_with_irqs_enabled();
    }
}

#[cfg(test)]
fn console_wait_for_input(_run_console_maintenance: bool) {
    CONSOLE_WAIT_TEST_COUNT.fetch_add(1, Ordering::SeqCst);
    if CONSOLE_WAIT_TEST_INJECT_NEWLINE.swap(false, Ordering::SeqCst) {
        process_console_input_byte(b'\n');
    }
}

#[cfg(test)]
pub fn push_console_input_for_tests(bytes: &[u8]) {
    let saved = crate::linux_driver_abi::tty::compat_termios();
    let mut quiet = saved;
    quiet.c_lflag &= !crate::linux_driver_abi::tty::LFLAG_ECHO;
    crate::linux_driver_abi::tty::set_compat_termios(quiet);
    for &byte in bytes {
        process_console_input_byte(byte);
    }
    crate::linux_driver_abi::tty::set_compat_termios(saved);
}

fn console_read(
    _file: &crate::fs::types::FileRef,
    buf: &mut [u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    if buf.is_empty() {
        return Ok(0);
    }

    loop {
        if let Some(n) = console_read_ready_or_signal(buf, pos)? {
            return Ok(n);
        }

        // Drain every input byte that is already available before returning.
        // Multi-byte key sequences (arrow/Home/End/Delete keys decode to
        // `ESC [ ...`) arrive in the input queue together, but the source
        // hands them back one byte per call. Returning a lone `ESC` makes a
        // raw-mode reader (bash readline) hit its ESC keyseq timeout and
        // mis-parse the rest as literal input, corrupting in-line editing.
        // Draining to exhaustion keeps the whole sequence in a single read().
        let mut drained = false;
        while let Some(input) = try_console_input() {
            process_console_input(input);
            drained = true;
        }
        if drained {
            if let Some(n) = console_read_ready_or_signal(buf, pos)? {
                return Ok(n);
            }
        } else if console_line_edit_pending() {
            // Keep draining RX while a canonical line is mid-entry. Even a
            // small TX/fbcon maintenance pass here can overrun QEMU's tiny
            // emulated UART FIFO during scripted or pasted input. Still yield
            // the CPU so an incomplete line cannot monopolize the kernel.
            console_wait_for_input(false);
        } else {
            console_wait_for_input(true);
        }
    }
}

fn console_write(
    _file: &crate::fs::types::FileRef,
    buf: &[u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    crate::kernel::console::write_bytes(buf);
    *pos = pos.saturating_add(buf.len() as u64);
    Ok(buf.len())
}

pub(crate) static CONSOLE_FILE_OPS: FileOps = FileOps {
    name: "console",
    read: Some(console_read),
    write: Some(console_write),
    llseek: None,
    fsync: None,
    poll: Some(console_poll),
    ioctl: Some(console_ioctl),
    mmap: None,
    release: None,
    readdir: None,
};

fn console_poll(_file: &crate::fs::types::FileRef) -> u32 {
    drain_console_control_bytes();
    #[cfg(not(test))]
    crate::kernel::console::maintenance_budgeted();
    let mut mask = crate::fs::select::POLLOUT as u32;
    if !CONSOLE_READY_BUFFER.lock().is_empty() {
        mask |= crate::fs::select::POLLIN as u32;
    }
    mask
}

fn console_ioctl(_file: &crate::fs::types::FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    if cmd == crate::linux_driver_abi::tty::TIOCGWINSZ {
        refresh_console_winsize();
    }
    crate::linux_driver_abi::tty::tty_ioctl_compat(cmd, arg)
}

fn refresh_console_winsize() {
    if let Some((cols, rows, xpixel, ypixel)) =
        crate::linux_driver_abi::video::fbdev::core::text_dimensions()
    {
        crate::linux_driver_abi::tty::set_compat_winsize(crate::linux_driver_abi::tty::Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: xpixel,
            ws_ypixel: ypixel,
        });
    }
}

pub fn bootstrap_initramfs_rootfs() -> Result<(), i32> {
    bootstrap_initramfs_rootfs_with_options(&BootOptions::default())
}

pub fn bootstrap_initramfs_rootfs_with_options(options: &BootOptions) -> Result<(), i32> {
    set_disk_root_device_alias(None);
    if let Some(hostname) = options.hostname.as_deref() {
        let _ = version::apply_hostname_param(hostname);
    }

    if !initramfs::is_installed() {
        if mount_disk_root_if_requested(options)?.is_some() {
            prepare_devtmpfs_mount()?;
            crate::init::boot_trace::record("devtmpfs", "populate start");
            populate_devtmpfs()?;
            crate::init::boot_trace::record("devtmpfs", "populate done");
            mount_boot_partition_if_available()?;
            mount_pseudo_filesystems()?;
            let _ = options.console_on_rootfs_ready(path_exists);
            load_configured_modules()?;
            return Ok(());
        }
    }

    bootstrap_rootfs()?;
    if initramfs::is_installed() {
        crate::init::boot_trace::record("initramfs", "materialize start");
        materialize_initramfs()?;
        crate::init::boot_trace::record("initramfs", "materialize done");
        crate::log_info!("", "VFS: Mounted root (rootfs filesystem) on device 0:1.");
    } else {
        populate_noinitramfs_rootfs()?;
        crate::log_info!("", "VFS: Mounted root (rootfs filesystem) on device 0:1.");
    }
    crate::init::boot_trace::record("devtmpfs", "populate start");
    populate_devtmpfs()?;
    crate::init::boot_trace::record("devtmpfs", "populate done");
    mount_boot_partition_if_available()?;
    mount_pseudo_filesystems()?;
    let _ = options.console_on_rootfs_ready(path_exists);
    load_configured_modules()?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiskRootMountSpec {
    pub source: String,
    pub fs_name: String,
    pub flags: u64,
    pub data: String,
}

pub fn disk_root_mount_spec(options: &BootOptions) -> Option<DiskRootMountSpec> {
    let source = options.root.as_deref()?.trim();
    if source.is_empty() {
        return None;
    }
    let fs_name = options
        .rootfstype
        .as_deref()
        .and_then(|value| {
            value
                .split(',')
                .map(str::trim)
                .find(|part| !part.is_empty())
        })
        .unwrap_or(DEFAULT_DISK_ROOT_FS);
    let flags = if options.root_readonly { MS_RDONLY } else { 0 };
    Some(DiskRootMountSpec {
        source: String::from(source),
        fs_name: String::from(fs_name),
        flags,
        data: options.rootflags.clone().unwrap_or_default(),
    })
}

pub fn mount_disk_root_if_requested(options: &BootOptions) -> Result<Option<Arc<Mount>>, i32> {
    let Some(mut spec) = disk_root_mount_spec(options) else {
        return Ok(None);
    };

    wait_for_disk_root_device(&mut spec)?;
    set_disk_root_device_alias(Some(&spec.source));
    fs::init();
    reset_mount_state();
    reset_console_buffers();
    crate::init::boot_trace::record("rootfs", "disk root mount start");

    let sb = match mount_fs(&spec.fs_name, &spec.source, spec.flags, &spec.data) {
        Ok(sb) => sb,
        Err(err) => {
            crate::log_warn!(
                "",
                "VFS: Cannot open root device \"{}\" as {}: error {}",
                spec.source,
                spec.fs_name,
                err
            );
            return Err(err);
        }
    };
    let root = sb.root().ok_or(EINVAL)?;
    let root_mount = Mount::alloc(sb, root, spec.flags as u32);
    set_rootfs(root_mount.clone());
    rebase_current_fs_to_namespace_root(&root_mount.root);

    let mode = if spec.flags & MS_RDONLY != 0 {
        "readonly"
    } else {
        "read-write"
    };
    crate::log_info!(
        "",
        "VFS: Mounted root ({} filesystem) {} on device {}.",
        spec.fs_name,
        mode,
        spec.source
    );
    crate::init::boot_trace::record("rootfs", "disk root mount done");
    Ok(Some(root_mount))
}

pub fn switch_to_disk_root_if_requested(options: &BootOptions) -> Result<bool, i32> {
    if disk_root_mount_spec(options).is_none() {
        return Ok(false);
    }

    // Linux `prepare_namespace()` mounts rootfs/initramfs first, lets driver
    // probing/module loading make the real root device appear, then pivots into
    // the mounted disk root. We reset the namespace to the disk root and rebuild
    // the early pseudo filesystems there.
    if mount_disk_root_if_requested(options)?.is_none() {
        return Ok(false);
    }
    prepare_devtmpfs_mount()?;
    crate::init::boot_trace::record("devtmpfs", "populate start");
    populate_devtmpfs()?;
    crate::init::boot_trace::record("devtmpfs", "populate done");
    mount_boot_partition_if_available()?;
    mount_pseudo_filesystems()?;
    let _ = options.console_on_rootfs_ready(path_exists);
    crate::log_info!("", "VFS: Pivoted into new rootfs");
    Ok(true)
}

fn disk_root_source_needs_block_wait(source: &str) -> bool {
    source.starts_with("/dev/") || disk_root_label(source).is_some()
}

fn disk_root_label(source: &str) -> Option<&str> {
    source
        .strip_prefix("LABEL=")
        .filter(|label| !label.is_empty())
}

fn normalized_dev_source(name: &str) -> String {
    let name = name.trim_start_matches("/dev/");
    format!("/dev/{name}")
}

fn disk_root_label_probe_rank(name: &str) -> u8 {
    let name = name.trim_start_matches("/dev/");
    if name.starts_with("vd") {
        0
    } else if name.starts_with("nvme") {
        1
    } else if name.starts_with("dm-") || name.starts_with("mapper/") {
        2
    } else if name.starts_with("xvd") {
        3
    } else if name.starts_with("sd") {
        4
    } else if name.starts_with("hd") {
        5
    } else if name.starts_with("sr") || name.starts_with("scd") || name.starts_with("loop") {
        8
    } else {
        6
    }
}

fn ext4_block_label_matches(
    bdev: &crate::block::block_device::BlockDeviceRef,
    label: &str,
) -> bool {
    crate::fs::ext4::super_block::read_super_identity(bdev)
        .ok()
        .and_then(|identity| identity.label)
        .as_deref()
        == Some(label)
}

fn resolve_disk_root_block_source(
    source: &str,
    fs_name: &str,
    trace_label_probe: bool,
) -> Option<String> {
    if source.starts_with("/dev/") {
        return crate::block::block_device::lookup_block_device(source)
            .filter(|bdev| bdev.capacity_sectors() > 0)
            .map(|_| String::from(source));
    }

    let label = disk_root_label(source)?;
    if fs_name != "ext4" {
        return None;
    }
    let mut devices = crate::block::block_device::registered_block_devices();
    devices.sort_by(|(left, _), (right, _)| {
        disk_root_label_probe_rank(left)
            .cmp(&disk_root_label_probe_rank(right))
            .then_with(|| left.cmp(right))
    });
    devices
        .into_iter()
        .find(|(name, bdev)| {
            if bdev.capacity_sectors() == 0 {
                return false;
            }
            if trace_label_probe {
                crate::log_info!(
                    "",
                    "VFS: probing root label {} on {}",
                    label,
                    normalized_dev_source(name)
                );
            }
            ext4_block_label_matches(bdev, label)
        })
        .map(|(name, _)| normalized_dev_source(&name))
}

fn disk_root_block_ready(source: &str, fs_name: &str) -> Option<String> {
    resolve_disk_root_block_source(source, fs_name, false)
}

fn set_disk_root_device_alias(source: Option<&str>) {
    *DISK_ROOT_DEVICE_ALIAS.lock() = source
        .filter(|source| source.starts_with("/dev/"))
        .map(String::from);
}

fn disk_root_device_alias() -> Option<String> {
    DISK_ROOT_DEVICE_ALIAS.lock().clone()
}

fn settle_disk_root_driver_events() {
    for _ in 0..DISK_ROOT_READY_SETTLE_POLLS {
        poll_disk_root_driver_events();
        core::hint::spin_loop();
    }
}

fn wait_for_disk_root_device(spec: &mut DiskRootMountSpec) -> Result<(), i32> {
    let requested = spec.source.clone();
    if let Some(resolved) = resolve_disk_root_block_source(&requested, &spec.fs_name, true) {
        if resolved != requested {
            crate::log_info!(
                "",
                "VFS: root device {} resolved to {}",
                requested,
                resolved
            );
        }
        spec.source = resolved;
        return Ok(());
    }
    if !disk_root_source_needs_block_wait(&requested) {
        return Ok(());
    }

    // Mirrors Linux `prepare_namespace()` in `vendor/linux/init/do_mounts.c`:
    // wait for driver probing/root bdev publication before trying the root mount.
    crate::log_info!("", "VFS: Waiting for root device {}...", requested);
    #[cfg(not(test))]
    let wait_deadline = crate::kernel::time::jiffies::jiffies().saturating_add(
        crate::kernel::time::jiffies::msecs_to_jiffies(DISK_ROOT_WAIT_MSECS),
    );
    #[cfg(test)]
    let wait_deadline = 0;
    let mut polls = 0usize;
    while !disk_root_wait_expired(polls, wait_deadline) {
        poll_disk_root_driver_events();
        if let Some(resolved) = disk_root_block_ready(&requested, &spec.fs_name) {
            if resolved != requested {
                crate::log_info!(
                    "",
                    "VFS: root device {} resolved to {}",
                    requested,
                    resolved
                );
            }
            spec.source = resolved;
            settle_disk_root_driver_events();
            return Ok(());
        }
        polls = polls.saturating_add(1);
    }

    crate::log_warn!(
        "",
        "VFS: root device {} did not appear; Linux disks: {:?}",
        requested,
        crate::linux_driver_abi::block::registered_linux_disk_names()
    );
    #[cfg(not(test))]
    crate::linux_driver_abi::storage_core::debug_dump_ahci_bar5("root device wait expired");
    Err(ENODEV)
}

fn poll_disk_root_driver_events() {
    #[cfg(test)]
    {
        core::hint::spin_loop();
    }
    #[cfg(not(test))]
    {
        let _ = crate::linux_driver_abi::poll_driver_abi_events();
        crate::kernel::console::maintenance_budgeted();
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
    }
}

fn disk_root_wait_expired(polls: usize, wait_deadline: u64) -> bool {
    #[cfg(test)]
    {
        let _ = wait_deadline;
        polls >= DISK_ROOT_WAIT_POLLS
    }
    #[cfg(not(test))]
    {
        let _ = polls;
        !crate::kernel::time::jiffies::time_before(
            crate::kernel::time::jiffies::jiffies(),
            wait_deadline,
        )
    }
}

pub fn remount_root_read_write() -> Result<(), i32> {
    mount::remount_mountpoint("/", MS_REMOUNT)
}

pub fn identify_legacy_ramdisk_image(bytes: &[u8], start_block: u32) -> LegacyRamdiskImage {
    match crate::init::do_mounts_rd::identify_ramdisk_image(bytes, start_block).kind {
        crate::init::do_mounts_rd::RamdiskImageKind::Gzip => LegacyRamdiskImage::Gzip,
        crate::init::do_mounts_rd::RamdiskImageKind::Bzip2 => LegacyRamdiskImage::Bzip2,
        crate::init::do_mounts_rd::RamdiskImageKind::Lzma => LegacyRamdiskImage::Lzma,
        crate::init::do_mounts_rd::RamdiskImageKind::Xz => LegacyRamdiskImage::Xz,
        crate::init::do_mounts_rd::RamdiskImageKind::Lzo => LegacyRamdiskImage::Lzo,
        crate::init::do_mounts_rd::RamdiskImageKind::Lz4 => LegacyRamdiskImage::Lz4,
        crate::init::do_mounts_rd::RamdiskImageKind::Romfs => LegacyRamdiskImage::Romfs,
        crate::init::do_mounts_rd::RamdiskImageKind::Cramfs => LegacyRamdiskImage::Cramfs,
        crate::init::do_mounts_rd::RamdiskImageKind::Squashfs => LegacyRamdiskImage::Squashfs,
        crate::init::do_mounts_rd::RamdiskImageKind::Minix => LegacyRamdiskImage::Minix,
        crate::init::do_mounts_rd::RamdiskImageKind::Ext2 => LegacyRamdiskImage::Ext2,
        crate::init::do_mounts_rd::RamdiskImageKind::Unknown => LegacyRamdiskImage::Unknown,
    }
}

pub fn legacy_initrd_load_outcome(options: &BootOptions, image: Option<&[u8]>) -> LegacyInitrdLoad {
    if options.noinitrd {
        return LegacyInitrdLoad::Disabled;
    }

    let Some(bytes) = image else {
        return LegacyInitrdLoad::NoImage;
    };

    match identify_legacy_ramdisk_image(bytes, options.ramdisk_start) {
        LegacyRamdiskImage::Unknown => LegacyInitrdLoad::InvalidImage { errno: ENOEXEC },
        image => LegacyInitrdLoad::Unsupported {
            image,
            errno: EOPNOTSUPP,
        },
    }
}

pub fn path_exists(path: &str) -> bool {
    path_walk(path).is_some()
}

pub fn read_rootfs_file(path: &str) -> Result<Vec<u8>, i32> {
    let dentry = path_walk(path).ok_or(ENOENT)?;
    let inode = dentry.inode().ok_or(ENOENT)?;
    let mut buf = vec![0u8; inode.size.load(Ordering::Acquire) as usize + 1];
    let file = alloc_file(
        dentry,
        O_RDONLY,
        inode.mode.load(Ordering::Acquire),
        inode.fops,
    );
    let n = vfs_read(&file, &mut buf)?;
    fput(file);
    buf.truncate(n);
    Ok(buf)
}

pub fn write_rootfs_file_at(path: &str, offset: u64, contents: &[u8]) -> Result<usize, i32> {
    let dentry = path_walk(path).ok_or(ENOENT)?;
    let inode = dentry.inode().ok_or(ENOENT)?;
    let file = alloc_file(
        dentry,
        O_RDWR,
        inode.mode.load(Ordering::Acquire),
        inode.fops,
    );
    let Some(write) = file.fops.write else {
        fput(file);
        return Err(EROFS);
    };
    let mut pos = offset;
    let result = write(&file, contents, &mut pos);
    fput(file);
    result
}

pub fn load_configured_modules() -> Result<(), i32> {
    let Ok(contents) = read_rootfs_file("/etc/modules") else {
        return Ok(());
    };
    let text = core::str::from_utf8(&contents).map_err(|_| EINVAL)?;
    for line in text.lines() {
        let name = line.split('#').next().unwrap_or("").trim();
        if name.is_empty() {
            continue;
        }
        crate::log_info!("", "modprobe: loading {}", name);
        match modprobe(name) {
            Ok(()) => crate::log_info!("", "modprobe: loaded {}", name),
            // A failed module load is logged but never fatal to PID1, mirroring
            // Linux: `systemd-modules-load`/`modprobe` report the error and the
            // boot continues (the kernel does not panic).  An essential driver
            // failing (e.g. the root-disk virtio_blk) surfaces later as a clear
            // mount failure rather than an opaque early panic here.
            // Ref: vendor/linux — modprobe(8) exit status is not wired to a
            // kernel panic; module_init failures propagate to userspace only.
            Err(errno) => crate::log_warn!(
                "",
                "modprobe: failed to load module {}: errno {} (continuing)",
                name,
                errno
            ),
        }
    }
    crate::log_info!("", "modprobe: configured module load complete");
    Ok(())
}

pub fn modprobe(module_name: &str) -> Result<(), i32> {
    if module::find_module(module_name).is_some() {
        return Ok(());
    }

    crate::linux_driver_abi::register_module_exports();
    let ko_path = resolve_module_path(module_name)?;
    let bytes = read_rootfs_file(&ko_path)?;
    let loaded = module::load_module(&bytes);

    match loaded {
        Ok(_) => Ok(()),
        Err(LoadModuleError::AlreadyLoaded) => Ok(()),
        Err(LoadModuleError::BadElf) => {
            crate::log_warn!("", "modprobe: {} rejected as bad ELF", module_name);
            Err(ENOEXEC)
        }
        Err(LoadModuleError::UndefinedSymbol(symbol)) => {
            crate::log_warn!(
                "",
                "modprobe: {} unresolved Linux module symbol {}",
                module_name,
                symbol
            );
            Err(ENOEXEC)
        }
        Err(LoadModuleError::UnsupportedReloc) => {
            crate::log_warn!(
                "",
                "modprobe: {} uses an unsupported ELF relocation",
                module_name
            );
            Err(ENOEXEC)
        }
        Err(LoadModuleError::InitFailed(errno)) => Err(errno),
        Err(LoadModuleError::Invalid) => Err(EINVAL),
    }
}

fn resolve_module_path(module_name: &str) -> Result<String, i32> {
    let dep = read_rootfs_file("/lib/modules/lupos/modules.dep")?;
    let text = core::str::from_utf8(&dep).map_err(|_| EINVAL)?;
    let normalized = normalize_module_name(module_name);

    for line in text.lines() {
        let Some((path, _deps)) = line.split_once(':') else {
            continue;
        };
        if module_path_matches_name(path, &normalized) {
            return Ok(alloc::format!("/lib/modules/lupos/{path}"));
        }
    }

    Err(ENOENT)
}

fn normalize_module_name(name: &str) -> String {
    name.trim_end_matches(".ko").replace('-', "_")
}

fn module_path_matches_name(path: &str, normalized_name: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    let stem = basename.trim_end_matches(".ko").replace('-', "_");
    stem == normalized_name
}

fn bootstrap_rootfs() -> Result<Arc<Mount>, i32> {
    fs::init();
    reset_mount_state();
    reset_console_buffers();
    crate::init::boot_trace::record("rootfs", "ramfs bootstrap start");

    let sb = mount_fs("ramfs", "", 0, "")?;
    let root = sb.root().ok_or(EINVAL)?;
    let root_mount = Mount::alloc(sb, root, 0);
    set_rootfs(root_mount.clone());
    rebase_current_fs_to_namespace_root(&root_mount.root);

    for dir in [
        "/bin",
        "/boot",
        "/dev",
        "/etc",
        "/etc/audit",
        "/etc/audit/plugins.d",
        "/etc/pam.d",
        "/etc/rc.d",
        "/etc/skel",
        "/etc/systemd",
        "/etc/systemd/journald.conf.d",
        "/etc/systemd/system",
        "/etc/systemd/system/getty.target.wants",
        "/etc/systemd/system/getty@tty1.service.d",
        "/etc/systemd/system/multi-user.target.wants",
        "/home",
        "/home/lupos",
        "/lib",
        "/lib64",
        "/lib/security",
        "/proc",
        "/root",
        "/run",
        // systemd probes /run/host/* for container-style host metadata and
        // copies /etc/os-release into /run/host/os-release.  Absence cascades
        // into "Failed to copy os-release for propagation" at PID 1 init.
        "/run/host",
        "/run/systemd",
        "/run/credentials",
        "/run/systemd/generator",
        "/run/systemd/generator.early",
        "/run/systemd/generator.late",
        "/sbin",
        "/sys",
        "/sys/fs",
        "/sys/fs/cgroup",
        "/tmp",
        "/usr",
        "/usr/bin",
        "/usr/lib",
        "/usr/lib/systemd",
        "/usr/lib/systemd/system",
        "/usr/libexec",
        "/usr/libexec/coreutils",
        "/usr/sbin",
        "/usr/share",
        "/var",
        "/var/log",
        "/var/log/audit",
    ] {
        ensure_dir(dir, DEFAULT_DIR_MODE)?;
    }
    set_path_metadata("/root", 0o700, 0, 0)?;
    set_path_metadata("/home/lupos", 0o755, 1000, 1000)?;
    set_path_metadata("/tmp", 0o1777, 0, 0)?;

    crate::init::boot_trace::record("rootfs", "ramfs bootstrap done");
    Ok(root_mount)
}

fn materialize_initramfs() -> Result<(), i32> {
    let mut dir_mtimes = Vec::new();
    let mut links: BTreeMap<(u32, u32, u32), Arc<Inode>> = BTreeMap::new();

    for entry in initramfs::installed_entries()? {
        if entry.is_dir() {
            ensure_dir(&entry.path, entry.mode & 0o7777)?;
            set_path_metadata_with_mtime(
                &entry.path,
                entry.mode & 0o7777,
                entry.uid(),
                entry.gid(),
                entry.mtime() as u64,
            )?;
            dir_mtimes.push((entry.path.clone(), entry.mtime() as u64));
            continue;
        }

        let hardlink_key = (entry.nlink() >= 2).then_some(entry.link_key());
        if let Some(key) = hardlink_key
            && let Some(existing) = links.get(&key)
        {
            link_existing_inode(&entry.path, existing)?;
            continue;
        }

        if entry.is_regular_file() {
            let contents = initramfs::read_file_slice(&entry.path)?;
            ensure_static_file(
                &entry.path,
                entry.mode & 0o7777,
                entry.uid(),
                entry.gid(),
                if entry.nlink() >= 2 { 1 } else { entry.nlink() },
                entry.mtime() as u64,
                contents,
            )?;
        } else if entry.is_symlink() {
            let target = initramfs::read_link(&entry.path)?;
            ensure_symlink_with_metadata(
                &entry.path,
                entry.mode & 0o7777,
                &target,
                entry.uid(),
                entry.gid(),
                if entry.nlink() >= 2 { 1 } else { entry.nlink() },
                entry.mtime() as u64,
            )?;
        } else if entry.is_chardev() || entry.is_blockdev() || entry.is_fifo() || entry.is_socket()
        {
            if is_kernel_owned_dev_path(&entry.path) {
                continue;
            }
            let kind = crate::fs::syscalls::mknod_kind(entry.mode)?;
            let fops = if kind == InodeKind::Blockdev {
                &crate::block::block_device::BLOCK_DEVICE_FILE_OPS
            } else {
                &RAMFS_FILE_OPS
            };
            create_special_node_with_metadata(
                &entry.path,
                kind,
                entry.mode & 0o7777,
                entry.uid(),
                entry.gid(),
                if entry.nlink() >= 2 { 1 } else { entry.nlink() },
                entry.mtime() as u64,
                fops,
            )?;
        }

        if let Some(key) = hardlink_key
            && let Some(inode) = path_walk(&entry.path).and_then(|dentry| dentry.inode())
        {
            links.entry(key).or_insert(inode);
        }
    }

    for (path, mtime) in dir_mtimes {
        set_path_mtime(&path, mtime)?;
    }
    Ok(())
}

fn link_existing_inode(path: &str, inode: &Arc<Inode>) -> Result<(), i32> {
    let (parent_path, leaf) = split_parent(path)?;
    let parent = ensure_dir(parent_path, DEFAULT_DIR_MODE)?;
    if d_lookup(&parent, leaf)
        .and_then(|dentry| dentry.inode())
        .is_some()
    {
        return Ok(());
    }

    let parent_inode = parent.inode().ok_or(EINVAL)?;
    match &parent_inode.private {
        InodePrivate::RamDir(children) => {
            children.lock().insert(String::from(leaf), inode.clone());
        }
        _ => return Err(EINVAL),
    }
    inode.nlink.fetch_add(1, Ordering::AcqRel);
    crate::fs::ramfs::dir_account_insert(&parent_inode);
    let child = d_alloc_child(&parent, leaf);
    child.instantiate(inode.clone());
    touch_inode_now(&parent_inode);
    Ok(())
}

fn ensure_static_file(
    path: &str,
    mode: u32,
    uid: u32,
    gid: u32,
    nlink: u32,
    mtime: u64,
    contents: &'static [u8],
) -> Result<(), i32> {
    let (parent_path, leaf) = split_parent(path)?;
    let parent = ensure_dir(parent_path, DEFAULT_DIR_MODE)?;

    if d_lookup(&parent, leaf).is_some() {
        return Ok(());
    }

    let parent_inode = parent.inode().ok_or(EINVAL)?;
    let sb = parent_inode.sb.lock().clone().ok_or(EINVAL)?;
    let inode = Inode::new(
        sb.alloc_ino(),
        InodeKind::Regular,
        mode,
        &RAMFS_FILE_INODE_OPS,
        &RAMFS_FILE_OPS,
        static_cow_bytes(contents),
    );
    inode.size.store(contents.len() as u64, Ordering::Release);
    init_inode_metadata(&inode, uid, gid, nlink, mtime);
    *inode.sb.lock() = Some(sb);

    match &parent_inode.private {
        InodePrivate::RamDir(children) => {
            children.lock().insert(String::from(leaf), inode.clone());
        }
        _ => return Err(EINVAL),
    }
    // Bypassing `ramfs_create` skips the parent's BOGO_DIRENT_SIZE bump,
    // so `ls -li` would report size=0 for every directory whose contents
    // came from the initramfs.  Mirror `vendor/linux/mm/shmem.c::
    // shmem_mknod`'s `dir->i_size += BOGO_DIRENT_SIZE` here.
    crate::fs::ramfs::dir_account_insert(&parent_inode);

    let child = d_alloc_child(&parent, leaf);
    child.instantiate(inode);
    touch_inode_now(&parent_inode);
    Ok(())
}

fn populate_noinitramfs_rootfs() -> Result<(), i32> {
    ensure_dir("/dev", DEFAULT_DIR_MODE)?;
    ensure_dir("/root", 0o700)?;
    create_special_node("/dev/console", InodeKind::Chardev, 0o600, &CONSOLE_FILE_OPS)?;
    Ok(())
}

fn populate_devtmpfs() -> Result<(), i32> {
    ensure_dir("/dev", DEFAULT_DIR_MODE)?;
    create_special_node("/dev/console", InodeKind::Chardev, 0o600, &CONSOLE_FILE_OPS)?;
    set_node_rdev("/dev/console", 5, 1);
    create_special_node("/dev/tty", InodeKind::Chardev, 0o666, &CONSOLE_FILE_OPS)?;
    set_node_rdev("/dev/tty", 5, 0);
    for (minor, tty) in [
        "/dev/tty0",
        "/dev/tty1",
        "/dev/tty2",
        "/dev/tty3",
        "/dev/tty4",
        "/dev/tty5",
        "/dev/tty6",
    ]
    .into_iter()
    .enumerate()
    {
        create_special_node(tty, InodeKind::Chardev, 0o620, &CONSOLE_FILE_OPS)?;
        // Virtual consoles live on TTY_MAJOR (4): tty0 is the current-VT alias,
        // tty1..tty6 the individual consoles.
        set_node_rdev(tty, 4, minor as u32);
    }
    create_special_node("/dev/ttyS0", InodeKind::Chardev, 0o620, &CONSOLE_FILE_OPS)?;
    // Serial console: TTY_MAJOR (4), minor 64 == ttyS0.
    set_node_rdev("/dev/ttyS0", 4, 64);
    ensure_dir("/dev/mapper", DEFAULT_DIR_MODE)?;
    create_special_node(
        "/dev/mapper/control",
        InodeKind::Chardev,
        0o600,
        &crate::block::dm::DM_CONTROL_FILE_OPS,
    )?;
    create_special_node("/dev/kmsg", InodeKind::Chardev, 0o666, &DEV_KMSG_FILE_OPS)?;
    for node in ["/dev/null", "/dev/zero", "/dev/random", "/dev/urandom"] {
        create_special_node(node, InodeKind::Chardev, 0o666, &RAMFS_FILE_OPS)?;
    }
    create_special_node("/dev/full", InodeKind::Chardev, 0o666, &DEV_FULL_FILE_OPS)?;
    // UNIX98 pty master multiplexor.  `/dev/ptmx` is char major 5 minor 2;
    // opening it allocates a pty pair and its `/dev/pts/N` slave node.
    replace_special_node(
        "/dev/ptmx",
        InodeKind::Chardev,
        0o666,
        &crate::linux_driver_abi::tty::pty::PTMX_FILE_OPS,
    )?;
    set_node_rdev("/dev/ptmx", 5, 2);
    ensure_dir("/dev/pts", DEFAULT_DIR_MODE)?;
    replace_special_node(
        "/dev/pts/ptmx",
        InodeKind::Chardev,
        0o666,
        &crate::linux_driver_abi::tty::pty::PTMX_FILE_OPS,
    )?;
    set_node_rdev("/dev/pts/ptmx", 5, 2);
    ensure_dir("/dev/hugepages", DEFAULT_DIR_MODE)?;
    ensure_dir("/dev/mqueue", DEFAULT_DIR_MODE)?;
    create_special_node(
        "/dev/vda",
        InodeKind::Blockdev,
        0o660,
        &crate::block::block_device::BLOCK_DEVICE_FILE_OPS,
    )?;
    create_special_node(
        "/dev/vda1",
        InodeKind::Blockdev,
        0o660,
        &crate::block::block_device::BLOCK_DEVICE_FILE_OPS,
    )?;
    populate_common_block_nodes()?;
    populate_registered_block_nodes()?;
    if let Some(root_device) = disk_root_device_alias() {
        ensure_symlink("/dev/root", 0o777, &root_device)?;
    }

    // POSIX fd-redirection convention shims.  Ref:
    // `vendor/linux/fs/devpts/inode.c` (devpts), `vendor/linux/init/main.c`
    // (`/dev/{stdin,stdout,stderr}` symlinks via tmpfiles.d), and
    // `vendor/systemd/systemd-260.1/tmpfiles.d/legacy.conf.in`.
    // Bash + login + GNU userspace assume these resolve before reading from
    // `&0`/`&1`/`&2` shell redirections.
    ensure_symlink("/dev/fd", 0o777, "/proc/self/fd")?;
    ensure_symlink("/dev/stdin", 0o777, "/proc/self/fd/0")?;
    ensure_symlink("/dev/stdout", 0o777, "/proc/self/fd/1")?;
    ensure_symlink("/dev/stderr", 0o777, "/proc/self/fd/2")?;
    populate_graphics_nodes()?;
    // Linux: vendor/linux/drivers/base/devtmpfs.c — devtmpfs_init.
    crate::log_info!("", "devtmpfs: mounted");
    Ok(())
}

fn populate_common_block_nodes() -> Result<(), i32> {
    for node in [
        "/dev/vda",
        "/dev/vda1",
        "/dev/vdb",
        "/dev/vdb1",
        "/dev/sda",
        "/dev/sda1",
        "/dev/sdb",
        "/dev/sdb1",
        "/dev/nvme0n1",
        "/dev/nvme0n1p1",
    ] {
        ensure_block_device_node(node, 0o660)?;
    }
    Ok(())
}

fn populate_registered_block_nodes() -> Result<(), i32> {
    for (name, _) in crate::block::block_device::registered_block_devices() {
        let path = normalized_dev_source(&name);
        ensure_block_device_node(&path, 0o660)?;
    }
    Ok(())
}

pub fn ensure_block_device_node(path: &str, mode: u32) -> Result<(), i32> {
    // Linux `init/do_mounts.h::create_dev` creates block-device nodes for
    // early root mounting; Lupos keeps the operation here with devtmpfs setup.
    create_special_node(
        path,
        InodeKind::Blockdev,
        mode,
        &crate::block::block_device::BLOCK_DEVICE_FILE_OPS,
    )
}

fn prepare_devtmpfs_mount() -> Result<(), i32> {
    ensure_dir("/dev", DEFAULT_DIR_MODE)?;
    if mount::rootfs()
        .map(|root| root.sb.fs_name != "ramfs")
        .unwrap_or(false)
    {
        let _ = do_mount("ramfs", "devtmpfs", "/dev", 0, "")?;
    }
    Ok(())
}

/// Stage the display-server-facing `/dev` nodes — `/dev/input/eventN` and
/// `/dev/fb0` — so X.Org and Weston can open them.  Idempotent.
fn populate_graphics_nodes() -> Result<(), i32> {
    use crate::linux_driver_abi::input::evdev_chardev::EVDEV_FILE_OPS;
    use crate::linux_driver_abi::input::register_default_evdev_devices;
    use crate::linux_driver_abi::video::fbdev::{FBDEV_FILE_OPS, fbdev_init};
    use crate::net::uevent::announce_class_device;

    register_default_evdev_devices();
    ensure_dir("/dev/input", 0o755)?;
    create_special_node(
        "/dev/input/event0",
        InodeKind::Chardev,
        0o660,
        &EVDEV_FILE_OPS,
    )?;
    create_special_node(
        "/dev/input/event1",
        InodeKind::Chardev,
        0o660,
        &EVDEV_FILE_OPS,
    )?;
    announce_class_device("input", "event0", "input", "input/event0");
    announce_class_device("input", "event1", "input", "input/event1");

    // evdev input nodes live on major 13 (INPUT_MAJOR): event0/event1 are
    // minors 64/65.
    set_node_rdev("/dev/input/event0", 13, 64);
    set_node_rdev("/dev/input/event1", 13, 65);
    if fbdev_init() {
        create_special_node("/dev/fb0", InodeKind::Chardev, 0o660, &FBDEV_FILE_OPS)?;
        set_node_rdev("/dev/fb0", 29, 0);
        announce_class_device("graphics", "fb0", "graphics", "fb0");
    }
    Ok(())
}

fn has_ext2_magic(image: &[u8]) -> bool {
    image
        .get(1024 + 56..1024 + 58)
        .is_some_and(|magic| magic == &[0x53, 0xef])
}

fn has_minix_magic(image: &[u8]) -> bool {
    let Some(magic) = image.get(1024 + 16..1024 + 18) else {
        return false;
    };
    matches!(u16::from_le_bytes([magic[0], magic[1]]), 0x137f | 0x138f)
}

fn mount_pseudo_filesystems() -> Result<(), i32> {
    ensure_dir("/proc", DEFAULT_DIR_MODE)?;
    ensure_dir("/sys", DEFAULT_DIR_MODE)?;
    let _ = do_mount("proc", "", "/proc", 0, "")?;
    crate::init::boot_trace::record("proc", "mounted");
    let _ = do_mount("sysfs", "", "/sys", 0, "")?;
    crate::init::boot_trace::record("sysfs", "mounted");
    ensure_dir("/sys/kernel/debug", DEFAULT_DIR_MODE)?;
    let _ = do_mount("debugfs", "debugfs", "/sys/kernel/debug", 0, "")?;
    crate::init::boot_trace::record("debugfs", "mounted");
    ensure_dir("/sys/fs", DEFAULT_DIR_MODE)?;
    ensure_dir("/sys/fs/cgroup", DEFAULT_DIR_MODE)?;
    let _ = do_mount("cgroup2", "", "/sys/fs/cgroup", 0, "")?;
    crate::init::boot_trace::record("cgroup2", "mounted");
    Ok(())
}

fn mount_boot_partition_if_available() -> Result<(), i32> {
    let mut disks = Vec::new();
    crate::block::gendisk::for_each(|disk| {
        disks.push((disk.name.clone(), disk.bdev.clone()));
    });

    let mut candidates = Vec::new();
    for (disk_name, disk_bdev) in disks {
        if let Ok(parts) =
            crate::block::partitions::register_partition_devices(&disk_name, &disk_bdev)
        {
            for part in parts {
                candidates.push(part.name);
            }
        }
    }

    for name in candidates {
        let source = alloc::format!("/dev/{name}");
        if do_mount("vfat", &source, "/boot", MS_RDONLY, "").is_ok()
            || do_mount("ext4", &source, "/boot", MS_RDONLY, "").is_ok()
        {
            crate::log_info!("", "Mounted /boot");
            crate::init::boot_trace::record("bootfs", "mounted /boot");
            return Ok(());
        }
    }

    Ok(())
}

#[cfg(any(test, feature = "test-boot-partition"))]
pub fn provision_test_boot_partition_disk(disk_name: &str) {
    use crate::block::block_device::BlockDevice;
    use crate::block::gendisk::register_gendisk;
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};
    use crate::block::partitions::mbr;
    use crate::fs::fat::fatent::FAT32_EOC;

    crate::block::init();
    let partition_start = 8usize;
    let mem = MemBlockDevice::new(disk_name, 128 * 512);
    {
        let mut data = mem.data.lock();
        mbr::build_mbr_with_one_partition(&mut data[..512], 0x0c, partition_start as u32, 64);

        let part = partition_start * 512;
        data[part + 11..part + 13].copy_from_slice(&512u16.to_le_bytes());
        data[part + 13] = 1;
        data[part + 14..part + 16].copy_from_slice(&1u16.to_le_bytes());
        data[part + 16] = 1;
        data[part + 32..part + 36].copy_from_slice(&64u32.to_le_bytes());
        data[part + 36..part + 40].copy_from_slice(&1u32.to_le_bytes());
        data[part + 44..part + 48].copy_from_slice(&2u32.to_le_bytes());

        let fat = part + 512;
        data[fat + 8..fat + 12].copy_from_slice(&FAT32_EOC.to_le_bytes());
        data[fat + 12..fat + 16].copy_from_slice(&FAT32_EOC.to_le_bytes());

        let root_dir = part + 2 * 512;
        data[root_dir..root_dir + 8].copy_from_slice(b"BOOT    ");
        data[root_dir + 8..root_dir + 11].copy_from_slice(b"TXT");
        data[root_dir + 11] = 0x20;
        data[root_dir + 26..root_dir + 28].copy_from_slice(&3u16.to_le_bytes());
        data[root_dir + 28..root_dir + 32].copy_from_slice(&5u32.to_le_bytes());

        let file_data = part + 3 * 512;
        data[file_data..file_data + 5].copy_from_slice(b"hello");
    }

    let disk = BlockDevice::wrap(mem, mem_block_device_ops());
    register_gendisk(disk_name, disk);
}

fn reset_mount_state() {
    *mount::MOUNTS.root.lock() = None;
    mount::MOUNTS.by_path.lock().clear();
}

fn rebase_current_fs_to_namespace_root(root: &DentryRef) {
    let fs = crate::fs::fs_struct::current_fs();
    if !fs.is_null() {
        let fs = unsafe { &*fs };
        crate::fs::fs_struct::set_fs_root(fs, root.clone());
        crate::fs::fs_struct::set_fs_pwd(fs, root.clone());
    }
    crate::fs::fs_struct::set_current_cwd_path("/");
}

fn ensure_dir(path: &str, mode: u32) -> Result<DentryRef, i32> {
    let root = mount::rootfs().ok_or(EINVAL)?.root.clone();
    if path == "/" || path.is_empty() {
        return Ok(root);
    }
    if let Some(existing) = path_walk(path) {
        return Ok(existing);
    }

    let mut cur = root;
    let mut prefix = String::new();
    for component in path.trim_matches('/').split('/').filter(|c| !c.is_empty()) {
        prefix.push('/');
        prefix.push_str(component);
        if let Some(existing) = path_walk(&prefix) {
            cur = existing;
            continue;
        }
        let parent_inode = cur.inode().ok_or(EINVAL)?;
        if let Some(next) = d_lookup(&cur, component) {
            if next.inode().is_some() {
                cur = next;
                continue;
            }
            let mkdir = parent_inode.ops.mkdir.ok_or(EINVAL)?;
            let inode = mkdir(&parent_inode, component, mode)?;
            next.instantiate(inode);
            cur = next;
            continue;
        }

        let mkdir = parent_inode.ops.mkdir.ok_or(EINVAL)?;
        let inode = mkdir(&parent_inode, component, mode)?;
        let child = d_alloc_child(&cur, component);
        child.instantiate(inode);
        cur = child;
    }

    Ok(cur)
}

fn ensure_regular_file(path: &str, mode: u32, contents: &[u8]) -> Result<(), i32> {
    let (parent_path, leaf) = split_parent(path)?;
    let parent = ensure_dir(parent_path, DEFAULT_DIR_MODE)?;

    let parent_inode = parent.inode().ok_or(EINVAL)?;
    let dentry = match d_lookup(&parent, leaf) {
        Some(existing) if existing.inode().is_some() => existing,
        Some(existing) => {
            let create = parent_inode.ops.create.ok_or(EINVAL)?;
            let inode = create(&parent_inode, leaf, mode)?;
            existing.instantiate(inode);
            existing
        }
        None => {
            let create = parent_inode.ops.create.ok_or(EINVAL)?;
            let inode = create(&parent_inode, leaf, mode)?;
            let child = d_alloc_child(&parent, leaf);
            child.instantiate(inode);
            child
        }
    };

    let inode = dentry.inode().ok_or(EINVAL)?;
    let file = alloc_file(dentry, O_RDWR, mode, inode.fops);
    let _ = vfs_write(&file, contents)?;
    fput(file);
    Ok(())
}

fn ensure_symlink(path: &str, mode: u32, target: &str) -> Result<(), i32> {
    ensure_symlink_with_metadata(path, mode, target, 0, 0, 1, 0)
}

fn ensure_symlink_with_metadata(
    path: &str,
    mode: u32,
    target: &str,
    uid: u32,
    gid: u32,
    nlink: u32,
    mtime: u64,
) -> Result<(), i32> {
    let (parent_path, leaf) = split_parent(path)?;
    let parent = ensure_dir(parent_path, DEFAULT_DIR_MODE)?;
    if let Some(existing) = d_lookup(&parent, leaf)
        && existing.inode().is_some()
    {
        return Ok(());
    }
    let parent_inode = parent.inode().ok_or(EINVAL)?;
    let inode = ramfs_symlink(&parent_inode, leaf, target, mode)?;
    init_inode_metadata(&inode, uid, gid, nlink, mtime);
    let child = d_lookup(&parent, leaf).unwrap_or_else(|| d_alloc_child(&parent, leaf));
    child.instantiate(inode);
    Ok(())
}

fn create_special_node(
    path: &str,
    kind: InodeKind,
    mode: u32,
    fops: &'static FileOps,
) -> Result<(), i32> {
    create_special_node_with_metadata(path, kind, mode, 0, 0, 1, 0, fops)
}

fn create_special_node_with_metadata(
    path: &str,
    kind: InodeKind,
    mode: u32,
    uid: u32,
    gid: u32,
    nlink: u32,
    mtime: u64,
    fops: &'static FileOps,
) -> Result<(), i32> {
    let (parent_path, leaf) = split_parent(path)?;
    let parent = ensure_dir(parent_path, DEFAULT_DIR_MODE)?;
    if d_lookup(&parent, leaf)
        .and_then(|dentry| dentry.inode())
        .is_some()
    {
        return Ok(());
    }

    let parent_inode = parent.inode().ok_or(EINVAL)?;
    let sb = parent_inode.sb.lock().clone().ok_or(EINVAL)?;
    let inode = Inode::new(
        sb.alloc_ino(),
        kind,
        mode,
        &RAMFS_FILE_INODE_OPS,
        fops,
        empty_ram_bytes(),
    );
    init_inode_metadata(&inode, uid, gid, nlink, mtime);
    *inode.sb.lock() = Some(sb);

    match &parent_inode.private {
        InodePrivate::RamDir(children) => {
            children.lock().insert(String::from(leaf), inode.clone());
        }
        _ => return Err(EINVAL),
    }
    // Same reasoning as `ensure_static_file`: bypassing `ramfs_create` /
    // `ramfs_mkdir` would leave parent dir size pinned at the empty
    // baseline.  Devtmpfs populates dozens of entries under /dev — they
    // need to count.
    crate::fs::ramfs::dir_account_insert(&parent_inode);

    let child = d_lookup(&parent, leaf).unwrap_or_else(|| d_alloc_child(&parent, leaf));
    child.instantiate(inode);
    touch_inode_now(&parent_inode);
    Ok(())
}

/// Remove a special node previously created under a ramfs `/dev` directory.
/// A no-op if the path (or its parent) does not resolve.  Mirrors the child
/// teardown `ramfs_unlink` performs: drop the RamDir entry, adjust the parent
/// directory accounting, and evict the dentry.
fn remove_special_node(path: &str) -> Result<(), i32> {
    let Ok((parent_path, leaf)) = split_parent(path) else {
        return Ok(());
    };
    let Some(parent) = path_walk(parent_path) else {
        return Ok(());
    };
    let Some(parent_inode) = parent.inode() else {
        return Ok(());
    };
    let removed = match &parent_inode.private {
        InodePrivate::RamDir(children) => children.lock().remove(leaf).is_some(),
        _ => false,
    };
    if removed {
        crate::fs::ramfs::dir_account_remove(&parent_inode);
        crate::fs::dcache::d_drop(&parent, leaf);
        touch_inode_now(&parent_inode);
    }
    Ok(())
}

/// Create a special node, replacing any existing entry of the same name so the
/// new `fops` take effect even if an earlier populator (e.g. an initramfs cpio)
/// already created a placeholder with different operations.
fn replace_special_node(
    path: &str,
    kind: InodeKind,
    mode: u32,
    fops: &'static FileOps,
) -> Result<(), i32> {
    remove_special_node(path)?;
    create_special_node(path, kind, mode, fops)
}

/// devpts: materialise `/dev/pts/<index>` for a freshly allocated pty slave.
/// Called from the pty master allocation path.  Linux's devpts creates this
/// node (char major 136) when `/dev/ptmx` hands out a new master.
pub(crate) fn devpts_create_slave(index: u32) -> Result<(), i32> {
    let path = alloc::format!("/dev/pts/{}", index);
    create_special_node(
        &path,
        InodeKind::Chardev,
        0o620,
        &crate::linux_driver_abi::tty::pty::PTS_SLAVE_FILE_OPS,
    )?;
    set_node_rdev(
        &path,
        crate::linux_driver_abi::tty::pty::UNIX98_PTY_SLAVE_MAJOR,
        index,
    );
    Ok(())
}

/// devpts: tear down `/dev/pts/<index>` when its pty pair is freed.
pub(crate) fn devpts_remove_slave(index: u32) {
    let path = alloc::format!("/dev/pts/{}", index);
    let _ = remove_special_node(&path);
}

/// Assign a device number (`st_rdev`) to an already-created special node.
///
/// Stored in Linux `new_encode_dev()` form so `stat(2)` reports the real
/// `major`/`minor`.  Userspace keys real behaviour off this: e.g. Xorg's
/// `xf86HasTTYs()` only enables VT/console management (opening the VT and
/// switching it to `KD_GRAPHICS`) when `major(stat("/dev/tty0").st_rdev)`
/// equals `TTY_MAJOR` (4).  A no-op if the path does not resolve.
fn set_node_rdev(path: &str, major: u32, minor: u32) {
    use crate::init::noinitramfs::{mkdev, new_encode_dev};
    if let Some(inode) = path_walk(path).and_then(|dentry| dentry.inode()) {
        inode.rdev.store(
            new_encode_dev(mkdev(major, minor)) as u64,
            Ordering::Release,
        );
    }
}

fn set_path_metadata(path: &str, mode: u32, uid: u32, gid: u32) -> Result<(), i32> {
    set_path_metadata_with_mtime(path, mode, uid, gid, 0)
}

fn set_path_metadata_with_mtime(
    path: &str,
    mode: u32,
    uid: u32,
    gid: u32,
    mtime: u64,
) -> Result<(), i32> {
    let dentry = path_walk(path).ok_or(ENOENT)?;
    let inode = dentry.inode().ok_or(EINVAL)?;
    inode
        .mode
        .store(mode | inode.kind.s_ifmt(), Ordering::Release);
    inode.uid.store(uid, Ordering::Release);
    inode.gid.store(gid, Ordering::Release);
    if mtime != 0 {
        inode.atime.store(mtime, Ordering::Release);
        inode.mtime.store(mtime, Ordering::Release);
        inode.ctime.store(mtime, Ordering::Release);
    }
    Ok(())
}

fn set_path_mtime(path: &str, mtime: u64) -> Result<(), i32> {
    if mtime == 0 {
        return Ok(());
    }
    let dentry = path_walk(path).ok_or(ENOENT)?;
    let inode = dentry.inode().ok_or(EINVAL)?;
    inode.atime.store(mtime, Ordering::Release);
    inode.mtime.store(mtime, Ordering::Release);
    inode.ctime.store(mtime, Ordering::Release);
    Ok(())
}

fn is_kernel_owned_dev_path(path: &str) -> bool {
    if !path.starts_with("/dev/") {
        return false;
    }
    if matches!(
        path,
        "/dev/console"
            | "/dev/tty"
            | "/dev/kmsg"
            | "/dev/null"
            | "/dev/zero"
            | "/dev/random"
            | "/dev/urandom"
            | "/dev/full"
            | "/dev/ptmx"
            | "/dev/mapper/control"
            | "/dev/vda"
            | "/dev/vda1"
            | "/dev/vdb"
            | "/dev/vdb1"
            | "/dev/sda"
            | "/dev/sda1"
            | "/dev/sdb"
            | "/dev/sdb1"
            | "/dev/nvme0n1"
            | "/dev/nvme0n1p1"
    ) {
        return true;
    }
    path.starts_with("/dev/tty")
        || path.starts_with("/dev/fb")
        || path.starts_with("/dev/input/")
        || path.starts_with("/dev/vd")
        || path.starts_with("/dev/sd")
        || path.starts_with("/dev/nvme")
}

fn split_parent(path: &str) -> Result<(&str, &str), i32> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return Err(EINVAL);
    }

    match trimmed.rsplit_once('/') {
        Some((parent, leaf)) => Ok((if parent.is_empty() { "/" } else { parent }, leaf)),
        None => Ok(("/", trimmed)),
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use crate::include::uapi::stat::{S_IFDIR, S_IFIFO, S_IFREG};
    use alloc::boxed::Box;
    use alloc::sync::Arc;
    use alloc::vec;
    use alloc::vec::Vec;

    #[derive(Clone, Copy)]
    struct HeaderSpec<'a> {
        name: &'a str,
        mode: u32,
        ino: u32,
        uid: u32,
        gid: u32,
        nlink: u32,
        mtime: u32,
        dev_major: u32,
        dev_minor: u32,
        rdev_major: u32,
        rdev_minor: u32,
        payload: &'a [u8],
    }

    fn append_header(out: &mut Vec<u8>, name: &str, mode: u32, payload: &[u8]) {
        append_header_full(
            out,
            HeaderSpec {
                name,
                mode,
                ino: 0,
                uid: 0,
                gid: 0,
                nlink: 1,
                mtime: 0,
                dev_major: 0,
                dev_minor: 0,
                rdev_major: 0,
                rdev_minor: 0,
                payload,
            },
        )
    }

    fn append_header_full(out: &mut Vec<u8>, spec: HeaderSpec<'_>) {
        fn write_hex(out: &mut Vec<u8>, value: u32) {
            let text = std::format!("{value:08x}");
            out.extend_from_slice(text.as_bytes());
        }

        out.extend_from_slice(b"070701");
        write_hex(out, spec.ino);
        write_hex(out, spec.mode);
        write_hex(out, spec.uid);
        write_hex(out, spec.gid);
        write_hex(out, spec.nlink);
        write_hex(out, spec.mtime);
        write_hex(out, spec.payload.len() as u32);
        write_hex(out, spec.dev_major);
        write_hex(out, spec.dev_minor);
        write_hex(out, spec.rdev_major);
        write_hex(out, spec.rdev_minor);
        write_hex(out, (spec.name.len() + 1) as u32);
        write_hex(out, 0);
        out.extend_from_slice(spec.name.as_bytes());
        out.push(0);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out.extend_from_slice(spec.payload);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }

    fn fixture_initramfs() -> &'static [u8] {
        let mut archive = Vec::new();
        append_header(
            &mut archive,
            "etc/inittab",
            0o100644,
            b"tty1::respawn:/sbin/init\n",
        );
        append_header(&mut archive, "etc/hostname", 0o100644, b"lupos\n");
        append_header(&mut archive, "etc/modules", 0o100644, b"");
        append_header(
            &mut archive,
            "lib/modules/lupos/modules.order",
            0o100644,
            b"",
        );
        append_header(&mut archive, "lib/modules/lupos/modules.dep", 0o100644, b"");
        append_header(&mut archive, "bin/busybox", 0o100755, b"ELF...");
        append_header(
            &mut archive,
            "usr/lib/systemd/systemd",
            0o100755,
            b"ELF-systemd",
        );
        append_header(
            &mut archive,
            "sbin/init",
            0o120777,
            b"/usr/lib/systemd/systemd",
        );
        append_header(&mut archive, "TRAILER!!!", 0, &[]);
        Box::leak(archive.into_boxed_slice())
    }

    fn missing_module_initramfs() -> &'static [u8] {
        let mut archive = Vec::new();
        append_header(&mut archive, "etc/modules", 0o100644, b"missing_net\n");
        append_header(
            &mut archive,
            "lib/modules/lupos/modules.dep",
            0o100644,
            b"kernel/drivers/net/virtio_net.ko:\n",
        );
        append_header(&mut archive, "TRAILER!!!", 0, &[]);
        Box::leak(archive.into_boxed_slice())
    }

    fn oversized_file_initramfs() -> &'static [u8] {
        let mut payload = vec![0x5au8; INITRAMFS_CONTIG_FILE_LIMIT + 1];
        payload[0] = b'L';
        payload[INITRAMFS_CONTIG_FILE_LIMIT] = b'Z';

        let mut archive = Vec::new();
        append_header(
            &mut archive,
            "usr/lib/x86_64-linux-gnu/systemd/libsystemd-shared-257.so",
            0o100755,
            &payload,
        );
        append_header(&mut archive, "TRAILER!!!", 0, &[]);
        Box::leak(archive.into_boxed_slice())
    }

    fn metadata_initramfs() -> &'static [u8] {
        let mut archive = Vec::new();
        append_header_full(
            &mut archive,
            HeaderSpec {
                name: "opt",
                mode: S_IFDIR | 0o1770,
                ino: 10,
                uid: 100,
                gid: 200,
                nlink: 2,
                mtime: 123,
                dev_major: 0,
                dev_minor: 1,
                rdev_major: 0,
                rdev_minor: 0,
                payload: &[],
            },
        );
        append_header_full(
            &mut archive,
            HeaderSpec {
                name: "opt/a",
                mode: S_IFREG | 0o4755,
                ino: 11,
                uid: 0,
                gid: 0,
                nlink: 2,
                mtime: 124,
                dev_major: 0,
                dev_minor: 1,
                rdev_major: 0,
                rdev_minor: 0,
                payload: &[],
            },
        );
        append_header_full(
            &mut archive,
            HeaderSpec {
                name: "opt/b",
                mode: S_IFREG | 0o4755,
                ino: 11,
                uid: 0,
                gid: 0,
                nlink: 2,
                mtime: 124,
                dev_major: 0,
                dev_minor: 1,
                rdev_major: 0,
                rdev_minor: 0,
                payload: b"hardlink payload",
            },
        );
        append_header_full(
            &mut archive,
            HeaderSpec {
                name: "run/initramfs-fifo",
                mode: S_IFIFO | 0o600,
                ino: 12,
                uid: 0,
                gid: 0,
                nlink: 1,
                mtime: 125,
                dev_major: 0,
                dev_minor: 1,
                rdev_major: 0,
                rdev_minor: 0,
                payload: &[],
            },
        );
        append_header(&mut archive, "dev/console", S_IFIFO | 0o777, &[]);
        append_header(&mut archive, "TRAILER!!!", 0, &[]);
        Box::leak(archive.into_boxed_slice())
    }

    #[test]
    fn disk_root_spec_defaults_to_ext4_readonly() {
        let options = BootOptions::parse("quiet root=/dev/vda");
        let spec = disk_root_mount_spec(&options).expect("disk root spec");

        assert_eq!(spec.source, "/dev/vda");
        assert_eq!(spec.fs_name, "ext4");
        assert_eq!(spec.flags, MS_RDONLY);
        assert_eq!(spec.data, "");
    }

    #[test]
    fn disk_root_spec_honors_rootfstype_rootflags_and_rw() {
        let options =
            BootOptions::parse("root=/dev/vda1 rootfstype=ext4,ext3 rootflags=noatime rw");
        let spec = disk_root_mount_spec(&options).expect("disk root spec");

        assert_eq!(spec.source, "/dev/vda1");
        assert_eq!(spec.fs_name, "ext4");
        assert_eq!(spec.flags, 0);
        assert_eq!(spec.data, "noatime");
    }

    #[test]
    fn disk_root_wait_reports_missing_block_device() {
        let mut spec = DiskRootMountSpec {
            source: String::from("/dev/missing-rootfs-test27"),
            fs_name: String::from("ext4"),
            flags: MS_RDONLY,
            data: String::new(),
        };

        assert_eq!(wait_for_disk_root_device(&mut spec), Err(ENODEV));
    }

    #[test]
    fn disk_root_label_resolves_registered_ext4_device() {
        unregister_test_root_fixtures();
        register_ext4_root_fixture("sda");
        let mut spec = DiskRootMountSpec {
            source: String::from("LABEL=lupos-root"),
            fs_name: String::from("ext4"),
            flags: MS_RDONLY,
            data: String::new(),
        };

        assert_eq!(wait_for_disk_root_device(&mut spec), Ok(()));
        assert_eq!(spec.source, "/dev/sda");
    }

    #[test]
    fn disk_root_label_probe_prefers_likely_disk_root_before_optical_devices() {
        let mut names = vec![
            String::from("sr0"),
            String::from("sda"),
            String::from("vda"),
            String::from("nvme0n1"),
        ];

        names.sort_by(|left, right| {
            disk_root_label_probe_rank(left)
                .cmp(&disk_root_label_probe_rank(right))
                .then_with(|| left.cmp(right))
        });

        assert_eq!(
            names,
            vec![
                String::from("vda"),
                String::from("nvme0n1"),
                String::from("sda"),
                String::from("sr0"),
            ]
        );
    }

    #[test]
    fn requested_disk_root_mount_replaces_namespace_root() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        let options = BootOptions::parse("root=none rootfstype=ramfs rw");

        let mounted = mount_disk_root_if_requested(&options)
            .expect("mount root")
            .expect("root requested");

        assert_eq!(mounted.sb.fs_name, "ramfs");
        assert!(!mounted.is_readonly());
        assert_eq!(mount::rootfs().expect("rootfs").sb.fs_name, "ramfs");
    }

    #[test]
    fn bootstrap_mounts_ext4_root_device_when_no_initramfs_is_installed() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        register_ext4_root_fixture("vda");

        let options = BootOptions::parse("root=/dev/vda rw");
        bootstrap_initramfs_rootfs_with_options(&options).expect("disk root bootstrap");

        let root = mount::rootfs().expect("rootfs");
        assert_eq!(root.sb.fs_name, "ext4");
        assert!(!root.is_readonly());
        assert!(path_exists("/dev/console"));
        assert!(path_exists("/dev/vda"));
        assert!(path_exists("/dev/root"));
        assert!(path_exists("/proc/self/stat"));
        assert!(path_exists("/sys/kernel"));
    }

    #[test]
    fn initramfs_can_switch_to_disk_root_and_remount_rw() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(fixture_initramfs()).expect("install initramfs");
        register_ext4_root_fixture("vda");

        let options = BootOptions::parse("root=/dev/vda rootfstype=ext4");
        bootstrap_initramfs_rootfs_with_options(&options).expect("initramfs bootstrap");
        assert_eq!(mount::rootfs().expect("ramfs root").sb.fs_name, "ramfs");
        assert_eq!(read_rootfs_file("/etc/hostname").unwrap(), b"lupos\n");

        assert!(
            switch_to_disk_root_if_requested(&options).expect("switch to disk root"),
            "root= should request disk-root switch"
        );
        let root = mount::rootfs().expect("disk root");
        assert_eq!(root.sb.fs_name, "ext4");
        assert!(root.is_readonly(), "root= without rw should mount ro");
        assert!(path_exists("/dev/console"));
        assert!(path_exists("/dev/vda"));
        assert!(path_exists("/dev/root"));
        assert!(path_exists("/proc/self/stat"));
        assert!(path_exists("/sys/kernel"));
        assert!(
            read_rootfs_file("/etc/hostname").is_err(),
            "initramfs files must not remain namespace root after disk switch"
        );

        remount_root_read_write().expect("remount / rw");
        assert!(!mount::rootfs().expect("remounted root").is_readonly());
    }

    #[test]
    fn disk_root_switch_rebases_current_fs_for_relative_cwd() {
        use crate::fs::fdtable::FilesStruct;
        use crate::include::uapi::fcntl::{AT_FDCWD, O_DIRECTORY};
        use crate::kernel::{cred::INIT_CRED, files, sched, task::TaskStruct};

        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(fixture_initramfs()).expect("install initramfs");
        register_ext4_root_fixture("vda");

        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 991;
        current.tgid = 991;
        current.cred = &raw const INIT_CRED;
        unsafe {
            files::set_task_files(&mut *current as *mut TaskStruct, FilesStruct::new());
            sched::set_current(&mut *current as *mut TaskStruct);
        }

        let options = BootOptions::parse("root=/dev/vda rootfstype=ext4");
        bootstrap_initramfs_rootfs_with_options(&options).expect("initramfs bootstrap");
        assert_eq!(mount::rootfs().expect("ramfs root").sb.fs_name, "ramfs");
        assert!(
            crate::fs::fs_struct::current_root_and_pwd().is_some(),
            "bootstrap should initialize current fs"
        );

        assert!(
            switch_to_disk_root_if_requested(&options).expect("switch to disk root"),
            "root= should request disk-root switch"
        );
        let root = mount::rootfs().expect("disk root");
        assert_eq!(root.sb.fs_name, "ext4");
        let (fs_root, fs_pwd) =
            crate::fs::fs_struct::current_root_and_pwd().expect("current fs after switch");
        assert!(Arc::ptr_eq(&fs_root, &root.root));
        assert!(Arc::ptr_eq(&fs_pwd, &root.root));

        unsafe {
            assert_eq!(crate::fs::syscalls::sys_chdir(b"/usr\0".as_ptr()), 0);
            let dot_fd = crate::fs::openat::sys_openat(
                AT_FDCWD,
                b".\0".as_ptr(),
                (O_RDONLY | O_DIRECTORY) as i32,
                0,
            );
            assert!(dot_fd >= 0);
            let mut dirents = [0u8; 512];
            let len = crate::fs::syscalls::sys_getdents64(
                dot_fd as i32,
                dirents.as_mut_ptr(),
                dirents.len(),
            );
            assert!(len > 0);
            let len = len as usize;
            assert!(dirents_contain(&dirents, len, b"bin"));
            assert!(
                !dirents_contain(&dirents, len, b"home"),
                "open('.') from /usr must not fall back to / after disk-root switch"
            );
            assert_eq!(
                crate::fs::syscalls::sys_chdir(b"home\0".as_ptr()),
                -(ENOENT as i64)
            );
            assert_eq!(crate::fs::syscalls::sys_chdir(b"bin\0".as_ptr()), 0);

            files::drop_task_files(&mut *current as *mut TaskStruct);
            sched::set_current(previous);
        }
    }

    fn dirents_contain(dirents: &[u8], len: usize, name: &[u8]) -> bool {
        let mut off = 0usize;
        while off < len {
            let reclen = u16::from_ne_bytes([dirents[off + 16], dirents[off + 17]]) as usize;
            assert!(reclen >= 20);
            let name_start = off + 19;
            let name_end = dirents[name_start..off + reclen]
                .iter()
                .position(|byte| *byte == 0)
                .map(|pos| name_start + pos)
                .expect("dirent nul");
            if &dirents[name_start..name_end] == name {
                return true;
            }
            off += reclen;
        }
        false
    }

    fn register_ext4_root_fixture(name: &str) {
        use crate::block::block_device::{
            BlockDevice, register_block_device, unregister_block_device,
        };
        use crate::block::gendisk::{register_gendisk, unregister_gendisk};
        use crate::block::mem::{MemBlockDevice, mem_block_device_ops};
        use crate::fs::ext4::extents::EXT4_EXT_MAGIC;
        use crate::fs::ext4::inode::OnDiskInode;
        use crate::include::uapi::stat::S_IFDIR;

        const BLOCK_SIZE: usize = 4096;
        const BLOCKS: usize = 16;
        const INODE_TABLE_BLOCK: usize = 5;
        const ROOT_DIR_BLOCK: usize = 6;
        const USR_DIR_BLOCK: usize = 7;
        const USR_BIN_DIR_BLOCK: usize = 8;
        const HOME_DIR_BLOCK: usize = 9;
        const HOME_LUPOS_DIR_BLOCK: usize = 10;

        let inode_size = core::mem::size_of::<OnDiskInode>();
        let mut image = alloc::vec![0u8; BLOCK_SIZE * BLOCKS];

        write_le_u32(&mut image, 1024, 16); // s_inodes_count
        write_le_u32(&mut image, 1028, BLOCKS as u32); // s_blocks_count_lo
        write_le_u32(&mut image, 1044, 0); // s_first_data_block
        write_le_u32(&mut image, 1048, 2); // s_log_block_size -> 4096
        write_le_u32(&mut image, 1056, 32768); // s_blocks_per_group
        write_le_u32(&mut image, 1064, 16); // s_inodes_per_group
        write_le_u16(&mut image, 1080, crate::fs::ext4::EXT4_SUPER_MAGIC);
        write_le_u32(&mut image, 1108, 11); // s_first_ino
        write_le_u16(&mut image, 1112, inode_size as u16); // s_inode_size
        image[1144..1154].copy_from_slice(b"lupos-root"); // s_volume_name

        let gd = BLOCK_SIZE;
        write_le_u32(&mut image, gd + 8, INODE_TABLE_BLOCK as u32);

        write_inode(
            &mut image,
            inode_size,
            2,
            raw_dir_inode(ROOT_DIR_BLOCK as u64, BLOCK_SIZE as u64),
        );
        for ino in 3..=5 {
            write_inode(&mut image, inode_size, ino, raw_dir_inode(0, 0));
        }
        write_inode(
            &mut image,
            inode_size,
            6,
            raw_dir_inode(USR_DIR_BLOCK as u64, BLOCK_SIZE as u64),
        );
        write_inode(
            &mut image,
            inode_size,
            7,
            raw_dir_inode(USR_BIN_DIR_BLOCK as u64, BLOCK_SIZE as u64),
        );
        write_inode(
            &mut image,
            inode_size,
            8,
            raw_dir_inode(HOME_DIR_BLOCK as u64, BLOCK_SIZE as u64),
        );
        write_inode(
            &mut image,
            inode_size,
            9,
            raw_dir_inode(HOME_LUPOS_DIR_BLOCK as u64, BLOCK_SIZE as u64),
        );

        let dir = ROOT_DIR_BLOCK * BLOCK_SIZE;
        write_dir_entry(&mut image[dir..dir + BLOCK_SIZE], 0, 2, ".", 2, 12);
        write_dir_entry(&mut image[dir..dir + BLOCK_SIZE], 12, 2, "..", 2, 12);
        write_dir_entry(&mut image[dir..dir + BLOCK_SIZE], 24, 3, "dev", 2, 12);
        write_dir_entry(&mut image[dir..dir + BLOCK_SIZE], 36, 4, "proc", 2, 12);
        write_dir_entry(&mut image[dir..dir + BLOCK_SIZE], 48, 5, "sys", 2, 12);
        write_dir_entry(&mut image[dir..dir + BLOCK_SIZE], 60, 6, "usr", 2, 12);
        write_dir_entry(
            &mut image[dir..dir + BLOCK_SIZE],
            72,
            8,
            "home",
            2,
            (BLOCK_SIZE - 72) as u16,
        );

        let usr_dir = USR_DIR_BLOCK * BLOCK_SIZE;
        write_dir_entry(&mut image[usr_dir..usr_dir + BLOCK_SIZE], 0, 6, ".", 2, 12);
        write_dir_entry(
            &mut image[usr_dir..usr_dir + BLOCK_SIZE],
            12,
            2,
            "..",
            2,
            12,
        );
        write_dir_entry(
            &mut image[usr_dir..usr_dir + BLOCK_SIZE],
            24,
            7,
            "bin",
            2,
            (BLOCK_SIZE - 24) as u16,
        );

        let usr_bin_dir = USR_BIN_DIR_BLOCK * BLOCK_SIZE;
        write_dir_entry(
            &mut image[usr_bin_dir..usr_bin_dir + BLOCK_SIZE],
            0,
            7,
            ".",
            2,
            12,
        );
        write_dir_entry(
            &mut image[usr_bin_dir..usr_bin_dir + BLOCK_SIZE],
            12,
            6,
            "..",
            2,
            (BLOCK_SIZE - 12) as u16,
        );

        let home_dir = HOME_DIR_BLOCK * BLOCK_SIZE;
        write_dir_entry(
            &mut image[home_dir..home_dir + BLOCK_SIZE],
            0,
            8,
            ".",
            2,
            12,
        );
        write_dir_entry(
            &mut image[home_dir..home_dir + BLOCK_SIZE],
            12,
            2,
            "..",
            2,
            12,
        );
        write_dir_entry(
            &mut image[home_dir..home_dir + BLOCK_SIZE],
            24,
            9,
            "lupos",
            2,
            (BLOCK_SIZE - 24) as u16,
        );

        let lupos_dir = HOME_LUPOS_DIR_BLOCK * BLOCK_SIZE;
        write_dir_entry(
            &mut image[lupos_dir..lupos_dir + BLOCK_SIZE],
            0,
            9,
            ".",
            2,
            12,
        );
        write_dir_entry(
            &mut image[lupos_dir..lupos_dir + BLOCK_SIZE],
            12,
            8,
            "..",
            2,
            (BLOCK_SIZE - 12) as u16,
        );

        let mem = MemBlockDevice::new(name, image.len());
        mem.data.lock().copy_from_slice(&image);
        let bdev = BlockDevice::wrap(mem, mem_block_device_ops());
        // Test isolation: a prior suite test may have left this device name
        // registered. Clear it first so registration is idempotent across the
        // full suite (the test passes in isolation but EBUSYs after a sibling).
        let _ = unregister_block_device(name);
        let _ = unregister_gendisk(name);
        register_block_device(name, bdev.clone()).expect("register test root block device");
        register_gendisk(name, bdev);

        fn write_le_u16(image: &mut [u8], off: usize, value: u16) {
            image[off..off + 2].copy_from_slice(&value.to_le_bytes());
        }

        fn write_le_u32(image: &mut [u8], off: usize, value: u32) {
            image[off..off + 4].copy_from_slice(&value.to_le_bytes());
        }

        fn write_inode(image: &mut [u8], inode_size: usize, ino: usize, raw: OnDiskInode) {
            let off = INODE_TABLE_BLOCK * BLOCK_SIZE + (ino - 1) * inode_size;
            let bytes = unsafe {
                core::slice::from_raw_parts(
                    (&raw as *const OnDiskInode).cast::<u8>(),
                    core::mem::size_of::<OnDiskInode>(),
                )
            };
            image[off..off + bytes.len()].copy_from_slice(bytes);
        }

        fn raw_dir_inode(phys_block: u64, size: u64) -> OnDiskInode {
            OnDiskInode {
                i_mode: ((S_IFDIR as u16) | 0o755).to_le(),
                i_uid: 0,
                i_size_lo: (size as u32).to_le(),
                i_atime: 0,
                i_ctime: 0,
                i_mtime: 0,
                i_dtime: 0,
                i_gid: 0,
                i_links_count: 2u16.to_le(),
                i_blocks_lo: ((size / 512) as u32).to_le(),
                i_flags: 0x80000u32.to_le(),
                _osd1: 0,
                i_block: extent_i_block(phys_block, if size == 0 { 0 } else { 1 }),
                i_generation: 0,
                i_file_acl_lo: 0,
                i_size_hi: ((size >> 32) as u32).to_le(),
                i_obso_faddr: 0,
                _osd2: [0; 12],
                i_extra_isize: 0,
                i_checksum_hi: 0,
                i_ctime_extra: 0,
                i_mtime_extra: 0,
                i_atime_extra: 0,
                i_crtime: 0,
                i_crtime_extra: 0,
                i_version_hi: 0,
                i_projid: 0,
            }
        }

        fn extent_i_block(phys_block: u64, len: u16) -> [u32; 15] {
            let mut bytes = [0u8; 60];
            bytes[0..2].copy_from_slice(&EXT4_EXT_MAGIC.to_le_bytes());
            bytes[2..4].copy_from_slice(&(if len == 0 { 0u16 } else { 1u16 }).to_le_bytes());
            bytes[4..6].copy_from_slice(&4u16.to_le_bytes());
            bytes[6..8].copy_from_slice(&0u16.to_le_bytes());
            if len != 0 {
                bytes[12..16].copy_from_slice(&0u32.to_le_bytes());
                bytes[16..18].copy_from_slice(&len.to_le_bytes());
                bytes[18..20].copy_from_slice(&((phys_block >> 32) as u16).to_le_bytes());
                bytes[20..24].copy_from_slice(&(phys_block as u32).to_le_bytes());
            }

            let mut out = [0u32; 15];
            for (slot, chunk) in out.iter_mut().zip(bytes.chunks_exact(4)) {
                *slot = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
            out
        }

        fn write_dir_entry(
            block: &mut [u8],
            off: usize,
            ino: u32,
            name: &str,
            file_type: u8,
            rec_len: u16,
        ) {
            block[off..off + 4].copy_from_slice(&ino.to_le_bytes());
            block[off + 4..off + 6].copy_from_slice(&rec_len.to_le_bytes());
            block[off + 6] = name.len() as u8;
            block[off + 7] = file_type;
            block[off + 8..off + 8 + name.len()].copy_from_slice(name.as_bytes());
        }
    }

    fn unregister_test_root_fixtures() {
        use crate::block::block_device::unregister_block_device;
        use crate::block::gendisk::unregister_gendisk;

        for name in [
            "vda",
            "vda1",
            "sda",
            "sda1",
            "nvme0n1",
            "nvme0n1p1",
            "bootfatunit",
        ] {
            let _ = unregister_block_device(name);
            let _ = unregister_gendisk(name);
        }
    }

    #[test]
    fn initramfs_rootfs_bootstrap_populates_expected_tree() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        unregister_test_root_fixtures();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(fixture_initramfs()).expect("install initramfs");
        bootstrap_initramfs_rootfs().expect("initramfs rootfs bootstrap");

        assert!(path_exists("/dev/console"));
        assert!(path_exists("/dev/tty6"));
        assert!(path_exists("/dev/hugepages"));
        assert!(path_exists("/dev/mqueue"));
        assert!(path_exists("/dev/vda1"));
        assert!(path_exists("/proc/self/stat"));
        assert!(path_exists("/proc/sys/fs/mqueue"));
        assert!(path_exists("/sys/kernel"));
        assert!(path_exists("/sys/kernel/debug"));
        assert!(path_exists("/sys/fs/cgroup/cgroup.controllers"));
        // systemd's `propagate_etc_os_release()` writes to /run/host/os-release;
        // the parent dir must exist or PID 1 logs "Failed to copy os-release".
        assert!(path_exists("/run/host"));
        assert!(path_exists("/etc/audit/plugins.d"));
        assert!(path_exists("/var/log/audit"));
        assert!(path_exists("/etc/inittab"));
        assert!(path_exists("/sbin/init"));
        assert!(path_exists("/usr/lib/systemd/systemd"));
        assert!(path_exists("/bin/busybox"));
        assert!(path_exists("/lib/modules/lupos/modules.order"));
        assert!(path_exists("/lib/modules/lupos/modules.dep"));
        assert_eq!(read_rootfs_file("/bin/busybox").unwrap(), b"ELF...");

        // `ls -li` on a directory unpacked from the initramfs must show
        // the tmpfs-style `BOGO_DIRENT_SIZE` accounting (baseline 40
        // bytes for `.` and `..`, +20 per child).  This guards against
        // the regression where `ensure_static_file` and
        // `create_special_node` inserted into the parent's `RamDir` map
        // without calling `dir_account_insert`, leaving `ls -li` to
        // report size=0 for every directory in the unpacked tree.
        // Ref: `vendor/linux/mm/shmem.c::shmem_mknod`.
        const BOGO: u64 = 20;
        let dev_inode = path_walk("/dev")
            .and_then(|d| d.inode())
            .expect("/dev inode");
        // Devtmpfs populates many entries — sanity floor: more than the
        // empty-dir baseline (2 * BOGO = 40).
        let dev_size = dev_inode.size.load(Ordering::Acquire);
        assert!(
            dev_size > 2 * BOGO,
            "/dev size {dev_size} must reflect populated devtmpfs entries (>{})",
            2 * BOGO
        );
        // /etc carries inittab + hostname + modules + ... from the
        // fixture initramfs.  Verify at least one child bumped it.
        let etc_inode = path_walk("/etc")
            .and_then(|d| d.inode())
            .expect("/etc inode");
        let etc_size = etc_inode.size.load(Ordering::Acquire);
        assert!(
            etc_size > 2 * BOGO,
            "/etc size {etc_size} must reflect initramfs files (>{})",
            2 * BOGO
        );

        // `/sbin/init` is a symlink to `/usr/lib/systemd/systemd`; use the
        // no-follow resolver so the assertion inspects the symlink dentry
        // itself rather than the regular file it points at.
        let (_, init_dentry) =
            crate::fs::mount::resolve_path_nofollow("/sbin/init").expect("init symlink resolves");
        let init = init_dentry.inode().expect("init symlink inode");
        assert_eq!(init.kind, InodeKind::Symlink);
        let mut target = [0u8; 64];
        let n = init.ops.readlink.unwrap()(&init, &mut target).unwrap();
        assert_eq!(&target[..n], b"/usr/lib/systemd/systemd");
        assert_eq!(read_rootfs_file("/etc/hostname").unwrap(), b"lupos\n");
        assert_eq!(read_rootfs_file("/etc/modules").unwrap(), b"");
        assert_eq!(
            read_rootfs_file("/lib/modules/lupos/modules.order").unwrap(),
            b""
        );
        let console = path_walk("/dev/console").expect("console");
        let inode = console.inode().expect("console inode");
        let file = alloc_file(console, O_RDWR, 0o600, inode.fops);
        reset_console_buffers();
        assert_eq!(
            console_poll(&file) & crate::fs::select::POLLIN as u32,
            0,
            "canonical tty input should not be readable before a line is complete"
        );
        push_console_input_for_tests(b"root");
        assert_eq!(
            console_poll(&file) & crate::fs::select::POLLIN as u32,
            0,
            "canonical tty input should wait for Enter"
        );
        push_console_input_for_tests(b"\n");
        assert_ne!(
            console_poll(&file) & crate::fs::select::POLLIN as u32,
            0,
            "canonical tty input should become readable after Enter"
        );
        reset_console_buffers();
        push_console_input_for_tests(b"root\nlupos\n");
        let mut input = [0u8; 8];
        let n = vfs_read(&file, &mut input).expect("console read");
        assert_eq!(&input[..n], b"root\n");
        let n = vfs_read(&file, &mut input).expect("console second read");
        assert_eq!(&input[..n], b"lupos\n");

        let mut termios = crate::linux_driver_abi::tty::KernelTermios::default();
        termios.c_lflag &= !(crate::linux_driver_abi::tty::LFLAG_ICANON
            | crate::linux_driver_abi::tty::LFLAG_ECHO);
        console_ioctl(
            &file,
            crate::linux_driver_abi::tty::TCSETS,
            &termios as *const _ as u64,
        )
        .expect("set raw-ish console termios");
        push_console_input_for_tests(b"\x1b[D");
        let mut raw = [0u8; 8];
        let n = vfs_read(&file, &mut raw).expect("raw console read");
        assert_eq!(&raw[..n], b"\x1b[D");

        // Regression: an arrow key reaches the input source as a multi-byte
        // ESC sequence that is handed back one byte per `try_console_input()`
        // call (the real i8042/serial path). `console_read` must drain the
        // whole sequence into a single read; otherwise a raw-mode reader
        // (bash readline) sees a lone ESC, times out, and mis-parses the rest
        // as literal input — corrupting in-line editing.
        push_hardware_input_for_tests(b"\x1b[D");
        let mut arrow = [0u8; 8];
        let n = vfs_read(&file, &mut arrow).expect("hardware arrow read");
        assert_eq!(
            &arrow[..n],
            b"\x1b[D",
            "multi-byte key sequence must arrive in one read, not split"
        );

        crate::kernel::console::reset_for_tests(12, 3);
        crate::kernel::console::write_visible_bytes(b"\x1b[18t\x1b[2;3H\x1b[6n");
        let mut report = [0u8; 32];
        let n = vfs_read(&file, &mut report).expect("terminal report read");
        assert_eq!(&report[..n], b"\x1b[8;3;12t\x1b[2;3R");

        let mut winsize = crate::linux_driver_abi::tty::Winsize::default();
        console_ioctl(
            &file,
            crate::linux_driver_abi::tty::TIOCGWINSZ,
            &mut winsize as *mut _ as u64,
        )
        .expect("get console winsize");
        assert!(winsize.ws_row >= 24);
        assert!(winsize.ws_col >= 80);
        reset_console_buffers();

        assert!(
            !crate::kernel::module::inserted_modules()
                .iter()
                .any(|name| name == "virtio_net"),
            "empty /etc/modules must not synthesize descriptor-backed drivers"
        );
    }

    #[test]
    fn console_line_edit_pending_only_tracks_partial_canonical_lines() {
        reset_console_buffers();
        crate::linux_driver_abi::tty::serial::clear_capture_for_tests();

        process_console_input_byte(b'e');
        assert!(console_line_edit_pending());
        process_console_input_byte(b'\n');
        assert!(!console_line_edit_pending());

        reset_console_buffers();
        let saved = crate::linux_driver_abi::tty::compat_termios();
        let mut raw = saved;
        raw.c_lflag &= !crate::linux_driver_abi::tty::LFLAG_ICANON;
        crate::linux_driver_abi::tty::set_compat_termios(raw);
        process_console_input_byte(b'x');
        assert!(!console_line_edit_pending());
        crate::linux_driver_abi::tty::set_compat_termios(saved);
        reset_console_buffers();
    }

    #[test]
    fn rootfs_bootstrap_creates_run_host_directory() {
        // systemd's `propagate_etc_os_release()` writes to /run/host/os-release;
        // the parent dir must exist or PID 1 logs "Failed to copy os-release for
        // propagation, ignoring: No such file or directory" during distro init.
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        let mut archive = Vec::new();
        append_header(&mut archive, "TRAILER!!!", 0, &[]);
        let bytes: &'static [u8] = Box::leak(archive.into_boxed_slice());
        initramfs::install_from_bytes(bytes).expect("install empty initramfs");

        bootstrap_rootfs().expect("rootfs bootstrap");

        let dentry = path_walk("/run/host").expect("/run/host must exist after bootstrap");
        let inode = dentry.inode().expect("/run/host inode");
        assert_eq!(inode.kind, InodeKind::Directory);
    }

    #[test]
    fn boot_partition_mounts_vfat_from_registered_partition() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        provision_test_boot_partition_disk("bootfatunit");

        bootstrap_rootfs().expect("rootfs bootstrap");
        mount_boot_partition_if_available().expect("boot partition mount");

        let dentry = path_walk("/boot/BOOT.TXT").expect("file from mounted /boot");
        let inode = dentry.inode().expect("boot file inode");
        let file = alloc_file(dentry, O_RDONLY, 0o644, inode.fops);
        let mut buf = [0u8; 5];
        let n = vfs_read(&file, &mut buf).expect("read boot file");
        fput(file);

        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");
    }

    #[test]
    fn initramfs_rootfs_bootstrap_keeps_files_static_cow() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(oversized_file_initramfs()).expect("install initramfs");

        bootstrap_initramfs_rootfs().expect("initramfs rootfs bootstrap");

        let dentry = path_walk("/usr/lib/x86_64-linux-gnu/systemd/libsystemd-shared-257.so")
            .expect("large staged library");
        let inode = dentry.inode().expect("large library inode");
        assert_eq!(
            inode.size.load(Ordering::Acquire) as usize,
            INITRAMFS_CONTIG_FILE_LIMIT + 1
        );
        let st = crate::fs::stat::vfs_getattr(&inode);
        assert!(st.ino > 0);
        assert!(st.mtime > 0);
        match &inode.private {
            InodePrivate::StaticCowBytes { base, overlay } => {
                assert_eq!(base.len(), INITRAMFS_CONTIG_FILE_LIMIT + 1);
                assert_eq!(base[0], b'L');
                assert_eq!(base[INITRAMFS_CONTIG_FILE_LIMIT], b'Z');
                assert!(overlay.lock().is_none());
            }
            _ => panic!("initramfs file should not be copied into a Vec at bootstrap"),
        }

        let file = alloc_file(dentry.clone(), O_RDONLY, 0o755, inode.fops);
        let mut first = [0u8; 1];
        let n = vfs_read(&file, &mut first).expect("read static payload");
        assert_eq!(n, 1);
        assert_eq!(first[0], b'L');

        let rw = alloc_file(dentry, O_RDWR, 0o755, inode.fops);
        let n = vfs_write(&rw, b"M").expect("first write promotes COW payload");
        assert_eq!(n, 1);
        match &inode.private {
            InodePrivate::StaticCowBytes { base, overlay } => {
                assert_eq!(base[0], b'L');
                let promoted = overlay.lock();
                let bytes = promoted.as_ref().expect("overlay after write");
                assert_eq!(bytes[0], b'M');
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn initramfs_materializer_preserves_metadata_specials_and_hardlinks() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(metadata_initramfs()).expect("install initramfs");

        bootstrap_initramfs_rootfs().expect("initramfs rootfs bootstrap");

        let opt = path_walk("/opt").and_then(|d| d.inode()).expect("/opt");
        assert_eq!(opt.kind, InodeKind::Directory);
        assert_eq!(opt.mode.load(Ordering::Acquire) & 0o7777, 0o1770);
        assert_eq!(opt.uid.load(Ordering::Acquire), 100);
        assert_eq!(opt.gid.load(Ordering::Acquire), 200);
        assert_eq!(opt.mtime.load(Ordering::Acquire), 123);

        assert_eq!(read_rootfs_file("/opt/a").unwrap(), b"hardlink payload");
        assert_eq!(read_rootfs_file("/opt/b").unwrap(), b"hardlink payload");
        let a = path_walk("/opt/a").and_then(|d| d.inode()).expect("/opt/a");
        let b = path_walk("/opt/b").and_then(|d| d.inode()).expect("/opt/b");
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(a.nlink.load(Ordering::Acquire), 2);
        assert_eq!(a.mode.load(Ordering::Acquire) & 0o7777, 0o4755);

        let fifo = path_walk("/run/initramfs-fifo")
            .and_then(|d| d.inode())
            .expect("fifo");
        assert_eq!(fifo.kind, InodeKind::Fifo);
        assert_eq!(fifo.mode.load(Ordering::Acquire) & 0o7777, 0o600);

        let console = path_walk("/dev/console")
            .and_then(|d| d.inode())
            .expect("devtmpfs console");
        assert_eq!(console.kind, InodeKind::Chardev);
        assert_eq!(console.mode.load(Ordering::Acquire) & 0o7777, 0o600);
    }

    #[test]
    fn initramfs_rootfs_bootstrap_completes_without_panicking() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(fixture_initramfs()).expect("install initramfs");

        bootstrap_initramfs_rootfs().expect("initramfs rootfs bootstrap");

        // Smoke: confirm the canonical pseudo-fs mounts were created.
        assert!(path_exists("/proc"));
        assert!(path_exists("/sys"));
        assert!(path_exists("/sys/kernel/debug"));
        assert!(path_exists("/dev/hugepages"));
        assert!(path_exists("/dev/mqueue"));
        assert!(path_exists("/dev/console"));
    }

    #[test]
    fn failed_module_load_logs_and_continues_without_panicking() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(missing_module_initramfs()).expect("install initramfs");

        bootstrap_initramfs_rootfs().expect("missing configured module should not abort boot");
        assert!(crate::kernel::module::find_module("missing_net").is_none());
    }

    #[test]
    fn modprobe_registers_driver_abi_exports_before_payload_resolution() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(missing_module_initramfs()).expect("install initramfs");

        bootstrap_initramfs_rootfs().expect("missing configured module should not abort boot");
        assert!(crate::kernel::module::find_symbol("device_initialize").is_some());
        assert!(crate::kernel::module::find_symbol("device_add").is_some());
        assert!(crate::kernel::module::find_symbol("device_register").is_some());
        assert!(crate::kernel::module::find_symbol("device_unregister").is_some());
        assert!(crate::kernel::module::find_symbol("driver_register").is_some());
        assert!(crate::kernel::module::find_symbol("driver_unregister").is_some());
        assert!(crate::kernel::module::find_symbol("__register_blkdev").is_some());
        assert!(crate::kernel::module::find_symbol("unregister_blkdev").is_some());
        assert!(crate::kernel::module::find_symbol("blk_mq_alloc_tag_set").is_some());
        assert!(crate::kernel::module::find_symbol("blk_mq_free_tag_set").is_some());
        assert!(crate::kernel::module::find_symbol("__blk_mq_alloc_disk").is_some());
        assert!(crate::kernel::module::find_symbol("device_add_disk").is_some());
        assert!(crate::kernel::module::find_symbol("put_disk").is_some());
        assert!(crate::kernel::module::find_symbol("del_gendisk").is_some());
        assert!(crate::kernel::module::find_symbol("set_disk_ro").is_some());
        assert!(crate::kernel::module::find_symbol("set_capacity").is_some());
        assert!(crate::kernel::module::find_symbol("set_capacity_and_notify").is_some());
        assert!(crate::kernel::module::find_symbol("__register_virtio_driver").is_some());
        assert!(crate::kernel::module::find_symbol("unregister_virtio_driver").is_some());
        assert!(
            crate::kernel::module::find_symbol("virtio_check_driver_offered_feature").is_some()
        );
        assert!(crate::kernel::module::find_symbol("virtio_config_changed").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_config_driver_disable").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_config_driver_enable").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_add_status").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_reset_device").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_find_vqs").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_find_single_vq").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_device_ready").is_some());
        assert!(crate::kernel::module::find_symbol("virtio_max_dma_size").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_add_sgs").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_kick_prepare").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_notify").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_kick").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_get_buf_ctx").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_get_buf").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_disable_cb").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_enable_cb_prepare").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_poll").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_enable_cb").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_enable_cb_delayed").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_detach_unused_buf").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_get_vring_size").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_is_broken").is_some());
        assert!(crate::kernel::module::find_symbol("virtqueue_dma_dev").is_some());
        assert!(crate::kernel::module::find_symbol("__virtqueue_break").is_some());
        assert!(crate::kernel::module::find_symbol("__virtqueue_unbreak").is_some());
        assert!(crate::kernel::module::find_symbol("register_virtio_device").is_some());
        assert!(crate::kernel::module::find_symbol("unregister_virtio_device").is_some());
        assert!(crate::kernel::module::find_symbol("is_virtio_device").is_some());
    }

    #[test]
    fn dev_full_rejects_writes_without_backing_storage() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(fixture_initramfs()).expect("install initramfs");
        bootstrap_initramfs_rootfs().expect("initramfs rootfs bootstrap");

        let dentry = path_walk("/dev/full").expect("/dev/full must exist");
        let inode = dentry.inode().expect("/dev/full inode");
        assert_eq!(inode.kind, InodeKind::Chardev);
        assert_eq!(inode.fops.name, "dev_full");

        let file = alloc_file(dentry, O_RDWR, 0o666, inode.fops);
        let size_before = inode.size.load(Ordering::Acquire);
        assert_eq!(vfs_write(&file, b"attacker data"), Err(ENOSPC));
        assert_eq!(inode.size.load(Ordering::Acquire), size_before);

        let mut buf = [0xff; 8];
        assert_eq!(vfs_read(&file, &mut buf), Ok(buf.len()));
        assert_eq!(buf, [0; 8]);
        assert_eq!(inode.size.load(Ordering::Acquire), size_before);
        fput(file);
    }

    /// Source-backed parity check for the devtmpfs surface that the
    /// systemd / agetty / login / bash chain expects.  References:
    ///   - vendor/linux/Documentation/admin-guide/devices.txt
    ///   - vendor/systemd/systemd-260.1/src/core/mount-setup.c
    ///     (`mount_table` — devtmpfs is mounted before anything else).
    ///   - vendor/systemd/systemd-260.1/tmpfiles.d/legacy.conf.in
    ///     (the `/dev/{fd,stdin,stdout,stderr}` POSIX symlinks).
    ///   - vendor/bash/bash-5.2.37/shell.c and `redir.c` (the fd-redirect
    ///     paths shell expands at `<&0`, `>&2`, etc.).
    #[test]
    fn devtmpfs_systemd_login_surface_is_complete() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(fixture_initramfs()).expect("install initramfs");
        bootstrap_initramfs_rootfs().expect("initramfs rootfs bootstrap");

        // Canonical tty / console chardevs.  Ref: devices.txt §"5 - tty".
        for node in [
            "/dev/console",
            "/dev/tty",
            "/dev/tty0",
            "/dev/tty1",
            "/dev/tty2",
            "/dev/tty3",
            "/dev/tty4",
            "/dev/tty5",
            "/dev/tty6",
            "/dev/ttyS0",
        ] {
            assert!(path_exists(node), "{node} must exist for getty/login chain");
        }

        // Standard data nodes (devices.txt §"1 mem").  /dev/full was missing
        // — GNU coreutils' `df`/`stat` tests + bash's `printf` write-error
        // exercise it.
        for node in [
            "/dev/null",
            "/dev/zero",
            "/dev/full",
            "/dev/random",
            "/dev/urandom",
            "/dev/kmsg",
        ] {
            let inode = path_walk(node)
                .and_then(|d| d.inode())
                .unwrap_or_else(|| panic!("{node} must exist"));
            assert_eq!(inode.kind, InodeKind::Chardev, "{node} must be a chardev");
            if node == "/dev/kmsg" {
                assert_eq!(inode.fops.name, "dev_kmsg");
                assert!(
                    inode.fops.poll.is_some(),
                    "/dev/kmsg must be pollable for journald"
                );
            }
        }

        // pty multiplexer + slave dir.  Ref: vendor/linux/fs/devpts/inode.c.
        assert!(path_exists("/dev/ptmx"));
        assert!(path_exists("/dev/pts"));
        assert!(path_exists("/dev/pts/ptmx"));
        assert!(path_exists("/dev/mapper/control"));

        // POSIX fd-redirection convention — bash, GNU coreutils, and login
        // dlopen these.  Each must be a symlink to the corresponding
        // /proc/self/fd path.  Use resolve_path_nofollow so we inspect the
        // symlink itself rather than the target it resolves to.
        for (link, target) in [
            ("/dev/fd", "/proc/self/fd"),
            ("/dev/stdin", "/proc/self/fd/0"),
            ("/dev/stdout", "/proc/self/fd/1"),
            ("/dev/stderr", "/proc/self/fd/2"),
        ] {
            let (_, dentry) = crate::fs::mount::resolve_path_nofollow(link)
                .unwrap_or_else(|err| panic!("{link} must resolve (errno {err})"));
            let inode = dentry
                .inode()
                .unwrap_or_else(|| panic!("{link} dentry has no inode"));
            assert_eq!(inode.kind, InodeKind::Symlink, "{link} must be a symlink");
            let readlink = inode.ops.readlink.expect("readlink op");
            let mut buf = [0u8; 64];
            let n = readlink(&inode, &mut buf).expect("readlink ok");
            assert_eq!(&buf[..n], target.as_bytes());
        }
    }

    /// Source-backed parity check that the kernel-side getty/login session
    /// machinery — setsid, setpgid, TIOCSCTTY, prepare_creds/commit_creds —
    /// is wired and addressable.  References:
    ///   - vendor/linux/kernel/sys.c (setsid/setpgid)
    ///   - vendor/linux/drivers/tty/tty_io.c::tty_ioctl (TIOCSCTTY)
    ///   - vendor/linux/kernel/cred.c (prepare_creds/commit_creds)
    #[test]
    fn login_session_controlling_tty_and_cred_apis_are_wired() {
        // setsid + setpgid round-trip from a fresh task — getty's first act
        // after fork() before it execs login is `setsid(); ioctl(TIOCSCTTY)`.
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut task = Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        task.pid = 4242;
        task.tgid = 4242;
        task.cred = &raw const crate::kernel::cred::INIT_CRED;
        unsafe { crate::kernel::sched::set_current(&mut *task) };
        crate::kernel::session::reset_for_tests();

        // setsid: makes the caller the leader of a new session/pgrp.
        let sid = unsafe { crate::kernel::session::sys_setsid() };
        assert_eq!(sid, 4242);
        // setpgid: idempotent on the session leader.
        let r = unsafe { crate::kernel::session::sys_setpgid(0, 0) };
        assert_eq!(r, 0);

        // TIOCSCTTY: the canonical "this is my controlling tty" ioctl from
        // util-linux/login-utils/login.c.
        crate::linux_driver_abi::tty::reset_compat_tty_state();
        let tty = crate::linux_driver_abi::tty::TtyStruct::new("tty1", 1);
        let result = tty.ioctl(crate::linux_driver_abi::tty::TIOCSCTTY, 4242);
        assert_eq!(result, 0, "TIOCSCTTY must succeed for a session leader");

        // Cred COW: prepare_creds → commit_creds is what setuid()/login()
        // use to swap into a less-privileged identity.
        let new = crate::kernel::cred::prepare_creds().expect("prepare_creds");
        crate::kernel::cred::commit_creds(new);

        unsafe { crate::kernel::sched::set_current(previous) };
    }

    #[test]
    fn login_hotkey_gate_allows_getty_and_login_tasks() {
        let previous = unsafe { crate::kernel::sched::get_current() };
        for comm in [
            *b"agetty\0\0\0\0\0\0\0\0\0\0",
            *b"login\0\0\0\0\0\0\0\0\0\0\0",
        ] {
            let mut task: crate::kernel::task::TaskStruct = unsafe { core::mem::zeroed() };
            task.comm = comm;
            unsafe { crate::kernel::sched::set_current(&mut task) };
            assert!(current_task_is_login_reader());
        }
        unsafe { crate::kernel::sched::set_current(previous) };
    }

    #[test]
    fn login_hotkey_gate_ignores_shell_tasks() {
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut task: crate::kernel::task::TaskStruct = unsafe { core::mem::zeroed() };
        task.comm = *b"bash\0\0\0\0\0\0\0\0\0\0\0\0";
        unsafe { crate::kernel::sched::set_current(&mut task) };
        assert!(!current_task_is_login_reader());
        unsafe { crate::kernel::sched::set_current(previous) };
    }

    #[test]
    fn console_signal_prefers_tty_foreground_process_group() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::linux_driver_abi::tty::reset_compat_tty_state();
        crate::kernel::session::reset_for_tests();
        crate::kernel::signal::reset_for_tests();
        reset_console_buffers();
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut shell = Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        shell.pid = 7100;
        shell.tgid = 7100;
        shell.cred = &raw const crate::kernel::cred::INIT_CRED;

        unsafe {
            crate::kernel::sched::set_current(&mut *shell);
            assert_eq!(crate::kernel::session::sys_setpgid(0, 0), 0);
            let child = crate::kernel::fork::copy_process(
                &mut *shell as *mut crate::kernel::task::TaskStruct,
                &crate::kernel::fork::KernelCloneArgs::default(),
            )
            .expect("copy foreground child");
            let child_pid = (*child).pid;
            assert_eq!(crate::kernel::session::sys_setpgid(child_pid, child_pid), 0);

            let fg = child_pid as u32;
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSPGRP,
                &fg as *const u32 as u64,
            )
            .expect("set tty foreground pgrp");
            deliver_console_signal(crate::kernel::signal::SIGINT);

            assert!(!crate::kernel::signal::has_pending_signal_for_pid(
                shell.pid,
                crate::kernel::signal::SIGINT
            ));
            assert!(crate::kernel::signal::has_pending_signal_for_pid(
                child_pid,
                crate::kernel::signal::SIGINT
            ));

            crate::kernel::exit::release_task(child);
            crate::linux_driver_abi::tty::reset_compat_tty_state();
            crate::kernel::session::reset_for_tests();
            crate::kernel::signal::reset_for_tests();
            crate::kernel::sched::set_current(previous);
        }
    }

    /// Linux n_tty matches signal chars against `c_cc[VINTR]`, not a
    /// hard-coded `0x03`.  `stty intr ^A` sets `c_cc[VINTR] = 0x01`;
    /// pressing ^A on the framebuffer console must then fire SIGINT to
    /// the foreground process group instead of being treated as a
    /// literal byte.  Ref: `vendor/linux/drivers/tty/n_tty.c::
    /// n_tty_receive_char_special` (the `INTR_CHAR(tty)` check).
    #[test]
    fn console_signal_honors_custom_c_cc_vintr_from_termios() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::linux_driver_abi::tty::reset_compat_tty_state();
        crate::kernel::session::reset_for_tests();
        crate::kernel::signal::reset_for_tests();
        reset_console_buffers();
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut shell = Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        shell.pid = 7300;
        shell.tgid = 7300;
        shell.cred = &raw const crate::kernel::cred::INIT_CRED;

        unsafe {
            crate::kernel::sched::set_current(&mut *shell);
            assert_eq!(crate::kernel::session::sys_setpgid(0, 0), 0);
            // Claim the controlling tty so deliver_console_signal won't
            // fall back to the current task.
            assert_eq!(crate::kernel::session::sys_setsid(), shell.pid as i64);
            let pgrp = shell.pid as u32;
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSCTTY,
                pgrp as u64,
            )
            .expect("TIOCSCTTY");
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSPGRP,
                &pgrp as *const u32 as u64,
            )
            .expect("TIOCSPGRP");

            // Rebind VINTR to ^A (0x01).  Pressing 0x01 must now fire
            // SIGINT; pressing 0x03 must NOT (it's just a literal byte).
            let mut termios = crate::linux_driver_abi::tty::KernelTermios::default();
            termios.c_cc[crate::linux_driver_abi::tty::VINTR] = 0x01;
            crate::linux_driver_abi::tty::set_compat_termios(termios);

            crate::kernel::signal::reset_for_tests();
            process_console_input_byte(0x01);
            assert!(
                crate::kernel::signal::has_pending_signal_for_pid(
                    shell.pid,
                    crate::kernel::signal::SIGINT
                ),
                "custom VINTR=^A must fire SIGINT"
            );

            crate::kernel::signal::reset_for_tests();
            process_console_input_byte(0x03);
            assert!(
                !crate::kernel::signal::has_pending_signal_for_pid(
                    shell.pid,
                    crate::kernel::signal::SIGINT
                ),
                "after `stty intr ^A` the literal ^C is no longer the signal char"
            );

            crate::linux_driver_abi::tty::reset_compat_tty_state();
            crate::kernel::session::reset_for_tests();
            crate::kernel::signal::reset_for_tests();
            reset_console_buffers();
            crate::kernel::sched::set_current(previous);
        }
    }

    /// Linux n_tty maps `^\` to SIGQUIT through `c_cc[VQUIT]`.
    /// Ref: `vendor/linux/drivers/tty/n_tty.c:1342-1344`.
    #[test]
    fn console_signal_delivers_sigquit_on_backslash() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::linux_driver_abi::tty::reset_compat_tty_state();
        crate::kernel::session::reset_for_tests();
        crate::kernel::signal::reset_for_tests();
        reset_console_buffers();
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut shell = Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        shell.pid = 7400;
        shell.tgid = 7400;
        shell.cred = &raw const crate::kernel::cred::INIT_CRED;

        unsafe {
            crate::kernel::sched::set_current(&mut *shell);
            assert_eq!(crate::kernel::session::sys_setpgid(0, 0), 0);
            assert_eq!(crate::kernel::session::sys_setsid(), shell.pid as i64);
            let pgrp = shell.pid as u32;
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSCTTY,
                pgrp as u64,
            )
            .expect("TIOCSCTTY");
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSPGRP,
                &pgrp as *const u32 as u64,
            )
            .expect("TIOCSPGRP");

            // Default termios: c_cc[VQUIT] = 0x1c (^\).
            process_console_input_byte(0x1c);
            assert!(
                crate::kernel::signal::has_pending_signal_for_pid(
                    shell.pid,
                    crate::kernel::signal::SIGQUIT
                ),
                "^\\ must fire SIGQUIT to the foreground pgrp"
            );

            crate::linux_driver_abi::tty::reset_compat_tty_state();
            crate::kernel::session::reset_for_tests();
            crate::kernel::signal::reset_for_tests();
            reset_console_buffers();
            crate::kernel::sched::set_current(previous);
        }
    }

    /// L_NOFLSH(tty) suppresses the input-buffer flush that normally
    /// follows an ISIG signal char.  Ref: `vendor/linux/drivers/tty/
    /// n_tty.c::isig` (the `L_NOFLSH(tty)` short-circuit at line 1071).
    #[test]
    fn console_signal_with_noflsh_preserves_canon_buffer() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::linux_driver_abi::tty::reset_compat_tty_state();
        crate::kernel::session::reset_for_tests();
        crate::kernel::signal::reset_for_tests();
        reset_console_buffers();
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut shell = Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        shell.pid = 7500;
        shell.tgid = 7500;
        shell.cred = &raw const crate::kernel::cred::INIT_CRED;

        unsafe {
            crate::kernel::sched::set_current(&mut *shell);
            assert_eq!(crate::kernel::session::sys_setpgid(0, 0), 0);
            assert_eq!(crate::kernel::session::sys_setsid(), shell.pid as i64);
            let pgrp = shell.pid as u32;
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSCTTY,
                pgrp as u64,
            )
            .expect("TIOCSCTTY");
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSPGRP,
                &pgrp as *const u32 as u64,
            )
            .expect("TIOCSPGRP");

            let mut termios = crate::linux_driver_abi::tty::KernelTermios::default();
            termios.c_lflag |= crate::linux_driver_abi::tty::LFLAG_NOFLSH;
            // Quiet echo so the test doesn't tickle the framebuffer.
            termios.c_lflag &= !crate::linux_driver_abi::tty::LFLAG_ECHO;
            crate::linux_driver_abi::tty::set_compat_termios(termios);

            // Stuff the canon buffer, then send ^C.
            CONSOLE_CANON_BUFFER.lock().extend_from_slice(b"abc");
            process_console_input_byte(0x03);
            assert_eq!(
                &*CONSOLE_CANON_BUFFER.lock(),
                b"abc",
                "L_NOFLSH must suppress the input flush"
            );
            assert!(crate::kernel::signal::has_pending_signal_for_pid(
                shell.pid,
                crate::kernel::signal::SIGINT
            ));

            crate::linux_driver_abi::tty::reset_compat_tty_state();
            crate::kernel::session::reset_for_tests();
            crate::kernel::signal::reset_for_tests();
            reset_console_buffers();
            crate::kernel::sched::set_current(previous);
        }
    }

    /// Once a session leader has claimed the controlling tty via
    /// `TIOCSCTTY`, `deliver_console_signal` must NOT fall back to the
    /// current task when the foreground pgrp is missing/empty — that's
    /// the divergence from Linux that would deliver SIGINT to a random
    /// process touching the console.  Linux's `__isig` simply does
    /// nothing if `tty_get_pgrp(tty)` returns NULL.  Ref:
    /// `vendor/linux/drivers/tty/n_tty.c::__isig`.
    #[test]
    fn console_signal_drops_when_no_foreground_pgrp_but_session_claimed() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        crate::linux_driver_abi::tty::reset_compat_tty_state();
        crate::kernel::session::reset_for_tests();
        crate::kernel::signal::reset_for_tests();
        reset_console_buffers();
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut leader =
            Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        leader.pid = 7600;
        leader.tgid = 7600;
        leader.cred = &raw const crate::kernel::cred::INIT_CRED;

        unsafe {
            crate::kernel::sched::set_current(&mut *leader);
            assert_eq!(crate::kernel::session::sys_setpgid(0, 0), 0);
            assert_eq!(crate::kernel::session::sys_setsid(), leader.pid as i64);
            // Claim controlling tty but DO NOT call TIOCSPGRP — leaves
            // COMPAT_TTY_PGRP at 0.  Linux would do nothing in this state;
            // we must match.
            let arg = leader.pid as u32;
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSCTTY,
                arg as u64,
            )
            .expect("TIOCSCTTY");
            // Force the pgrp slot back to 0 to simulate the "no foreground
            // pgrp" state described by Linux's tty_get_pgrp(tty) → NULL.
            let zero: u32 = 0;
            crate::linux_driver_abi::tty::tty_ioctl_compat(
                crate::linux_driver_abi::tty::TIOCSPGRP,
                &zero as *const u32 as u64,
            )
            .expect("TIOCSPGRP zero");

            deliver_console_signal(crate::kernel::signal::SIGINT);
            assert!(
                !crate::kernel::signal::has_pending_signal_for_pid(
                    leader.pid,
                    crate::kernel::signal::SIGINT
                ),
                "session active + empty pgrp must drop the signal, not deliver to current"
            );

            crate::linux_driver_abi::tty::reset_compat_tty_state();
            crate::kernel::session::reset_for_tests();
            crate::kernel::signal::reset_for_tests();
            crate::kernel::sched::set_current(previous);
        }
    }

    #[test]
    fn console_read_yields_when_no_input_is_ready() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        reset_console_buffers();
        CONSOLE_WAIT_TEST_INJECT_NEWLINE.store(true, Ordering::SeqCst);

        let file = alloc_file(
            crate::fs::dcache::d_alloc("tty"),
            O_RDONLY,
            0,
            &CONSOLE_FILE_OPS,
        );
        let mut buf = [0u8; 8];
        let mut pos = 0u64;
        assert_eq!(console_read(&file, &mut buf, &mut pos), Ok(1));
        assert_eq!(&buf[..1], b"\n");
        assert_eq!(pos, 1);
        assert_eq!(CONSOLE_WAIT_TEST_COUNT.load(Ordering::SeqCst), 1);

        reset_console_buffers();
    }

    #[test]
    fn console_read_returns_raw_byte_before_pending_signal_interrupt() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        reset_console_buffers();
        crate::kernel::signal::reset_for_tests();
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut task = Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        task.pid = 7200;
        task.tgid = 7200;
        task.cred = &raw const crate::kernel::cred::INIT_CRED;

        unsafe {
            crate::kernel::sched::set_current(&mut *task);

            let mut termios = crate::linux_driver_abi::tty::KernelTermios::default();
            termios.c_lflag &= !(crate::linux_driver_abi::tty::LFLAG_ICANON
                | crate::linux_driver_abi::tty::LFLAG_ECHO);
            crate::linux_driver_abi::tty::set_compat_termios(termios);
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *task as *mut crate::kernel::task::TaskStruct,
                    crate::kernel::signal::SIGCHLD
                ),
                0
            );

            process_console_input(ConsoleInput::Byte(b'f'));
            let mut buf = [0u8; 8];
            let mut pos = 0u64;
            assert_eq!(
                console_read_ready_or_signal(&mut buf, &mut pos).expect("raw byte first"),
                Some(1)
            );
            assert_eq!(&buf[..1], b"f");
            assert_eq!(pos, 1);

            reset_console_buffers();
            crate::kernel::signal::reset_for_tests();
            crate::kernel::sched::set_current(previous);
        }
    }

    #[test]
    fn console_read_reports_eintr_only_without_ready_bytes() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        reset_console_buffers();
        crate::kernel::signal::reset_for_tests();
        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut task = Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        task.pid = 7201;
        task.tgid = 7201;
        task.cred = &raw const crate::kernel::cred::INIT_CRED;

        unsafe {
            crate::kernel::sched::set_current(&mut *task);
            assert_eq!(
                crate::kernel::signal::send_signal_to_task(
                    &mut *task as *mut crate::kernel::task::TaskStruct,
                    crate::kernel::signal::SIGCHLD
                ),
                0
            );

            let mut buf = [0u8; 8];
            let mut pos = 0u64;
            assert_eq!(
                console_read_ready_or_signal(&mut buf, &mut pos),
                Err(crate::include::uapi::errno::EINTR)
            );
            assert_eq!(pos, 0);

            reset_console_buffers();
            crate::kernel::signal::reset_for_tests();
            crate::kernel::sched::set_current(previous);
        }
    }

    #[test]
    fn initramfs_rootfs_bootstrap_is_repeatable() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        initramfs::install_from_bytes(fixture_initramfs()).expect("install initramfs");
        bootstrap_initramfs_rootfs().expect("first bootstrap");
        bootstrap_initramfs_rootfs().expect("second bootstrap");
        assert!(path_exists("/proc/meminfo"));
        assert!(path_exists("/sys/kernel"));
    }

    #[test]
    fn noinitramfs_fallback_creates_minimal_rootfs() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        bootstrap_initramfs_rootfs_with_options(&BootOptions::default())
            .expect("initramfs rootfs bootstrap");

        assert!(path_exists("/dev"));
        assert!(path_exists("/dev/console"));
        assert!(path_exists("/root"));
        assert!(path_exists("/proc/self/stat"));
        assert!(path_exists("/sys/kernel"));
    }

    #[test]
    fn boot_options_apply_hostname() {
        let _guard = crate::fs::mount::TEST_MOUNT_LOCK.lock();
        initramfs::reset_for_tests();
        let options = BootOptions::parse("hostname=rootfs-node");
        bootstrap_initramfs_rootfs_with_options(&options).expect("initramfs rootfs bootstrap");

        let nodename = crate::kernel::utsname::current_nodename();
        assert_eq!(&nodename[..11], b"rootfs-node");
    }

    #[test]
    fn legacy_ramdisk_magic_detection_matches_linux_headers() {
        assert_eq!(
            identify_legacy_ramdisk_image(b"\x1f\x8bpayload", 0),
            LegacyRamdiskImage::Gzip
        );
        assert_eq!(
            identify_legacy_ramdisk_image(b"\xfd7zXZ\x00payload", 0),
            LegacyRamdiskImage::Xz
        );
        let mut padded_cramfs = vec![0u8; 0x204];
        padded_cramfs[0x200..0x204].copy_from_slice(&CRAMFS_MAGIC_LE);
        assert_eq!(
            identify_legacy_ramdisk_image(&padded_cramfs, 0),
            LegacyRamdiskImage::Cramfs
        );

        let mut ext2 = vec![0u8; 2048];
        ext2[1024 + 56] = 0x53;
        ext2[1024 + 57] = 0xef;
        assert_eq!(
            identify_legacy_ramdisk_image(&ext2, 0),
            LegacyRamdiskImage::Ext2
        );

        let mut minix_and_ext2 = vec![0u8; 2048];
        minix_and_ext2[1024 + 16..1024 + 18].copy_from_slice(&0x137fu16.to_le_bytes());
        minix_and_ext2[1024 + 56] = 0x53;
        minix_and_ext2[1024 + 57] = 0xef;
        assert_eq!(
            identify_legacy_ramdisk_image(&minix_and_ext2, 0),
            LegacyRamdiskImage::Minix
        );

        let mut shifted = vec![0u8; LINUX_INITRD_BLOCK_SIZE + 8];
        shifted[LINUX_INITRD_BLOCK_SIZE..LINUX_INITRD_BLOCK_SIZE + 3].copy_from_slice(b"BZh");
        assert_eq!(
            identify_legacy_ramdisk_image(&shifted, 1),
            LegacyRamdiskImage::Bzip2
        );
    }

    #[test]
    fn legacy_initrd_outcome_reports_disabled_and_unsupported() {
        let disabled = BootOptions::parse("noinitrd");
        assert_eq!(
            legacy_initrd_load_outcome(&disabled, Some(b"\x1f\x8bpayload")),
            LegacyInitrdLoad::Disabled
        );

        let options = BootOptions::parse("ramdisk_start=0");
        assert_eq!(
            legacy_initrd_load_outcome(&options, None),
            LegacyInitrdLoad::NoImage
        );
        assert_eq!(
            legacy_initrd_load_outcome(&options, Some(b"not-a-ramdisk")),
            LegacyInitrdLoad::InvalidImage { errno: ENOEXEC }
        );
        assert_eq!(
            legacy_initrd_load_outcome(&options, Some(b"\x1f\x8bpayload")),
            LegacyInitrdLoad::Unsupported {
                image: LegacyRamdiskImage::Gzip,
                errno: EOPNOTSUPP,
            }
        );
    }
}
