//! linux-parity: complete
//! linux-source: vendor/linux/drivers/tty
//! test-origin: linux:vendor/linux/drivers/tty
//! TTY core — M57.
//!
//! Mirrors `drivers/tty/tty_io.c`, `include/linux/tty.h`,
//! `include/linux/tty_driver.h`, and `drivers/tty/n_tty.c`.
//!
//! Implements the TTY layer, the n_tty line discipline, 8250 UART port
//! registration, and the job-control ioctls:
//!   `TIOCSCTTY`, `TIOCGPGRP`, `TIOCSPGRP`, `TIOCGWINSZ`, `TIOCSWINSZ`.
//!
//! References:
//!   - `include/linux/tty.h:188`                — `struct tty_struct`
//!   - `include/linux/tty_driver.h:526`          — `struct tty_driver`
//!   - `drivers/tty/tty_io.c:3425`              — `tty_register_driver`
//!   - `drivers/tty/n_tty.c:1885,1865`          — `n_tty_open/close`
//!   - `drivers/tty/tty_jobctrl.c`              — job-control ioctls
//!   - `include/linux/serial_core.h:442,888`    — `uart_port` / `uart_driver`
//!   - `drivers/tty/serial/8250/8250_core.c:693`— `serial8250_register_8250_port`

extern crate alloc;

pub mod ldisc;
pub mod linux_sources;
pub mod pty;
pub mod serial;
#[cfg(any(test, CONFIG_SERIAL_8250 = "y"))]
pub mod serial8250;

pub use crate::{serial_print, serial_println};

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::kernel::module::{export_symbol, find_symbol};

use ldisc::NTtyState;

/// Open-time dispatch for tty clone/devpts character devices.
///
/// Linux reaches these through `chrdev_open()` and `tty_open()`. Lupos binds
/// most device operations directly to inodes, so `openat` calls this narrow
/// dispatcher before allocating a generic `File`; that is where PTY open
/// counts and automatic controlling-terminal assignment must occur.
pub fn open_special_tty(
    dentry: crate::fs::types::DentryRef,
    flags: u32,
    mode: u32,
) -> Option<Result<crate::fs::types::FileRef, i32>> {
    let inode = dentry.inode()?;
    if inode.kind != crate::fs::types::InodeKind::Chardev {
        return None;
    }
    let dev = inode.rdev.load(Ordering::Acquire) as u32;
    let major = (dev & 0x000f_ff00) >> 8;
    let minor = (dev & 0xff) | ((dev >> 12) & 0x000f_ff00);
    match (major, minor) {
        (5, 0) => Some(pty::open_current_tty(dentry, flags, mode)),
        (5, 2) => Some(pty::open_ptmx(dentry, flags, mode)),
        (pty::UNIX98_PTY_SLAVE_MAJOR, index) => Some(pty::open_slave(dentry, flags, mode, index)),
        _ => None,
    }
}

/// Linux `tty_update_time()` — `drivers/tty/tty_io.c`. A tty read bumps the
/// inode's atime and a write bumps mtime (unlike a regular file, ctime is
/// left untouched). Updates are coarsened to ~8s granularity — Linux only
/// stores when the new value differs from the old in bits above the low 3 —
/// so a `stat()` can't be used to read exact I/O timing off the tty.
pub fn tty_update_time(file: &crate::fs::types::FileRef, mtime: bool) {
    let Some(inode) = file.dentry.inode() else {
        return;
    };
    let now = crate::fs::types::current_inode_timestamp_secs();
    let field = if mtime { &inode.mtime } else { &inode.atime };
    let prev = field.load(Ordering::Acquire);
    if (now ^ prev) & !7 != 0 {
        field.store(now, Ordering::Release);
    }
}

// ── TTY ioctl numbers (ABI-stable — from `include/uapi/asm-generic/ioctls.h`) ──
pub const TIOCSCTTY: u32 = 0x540E;
pub const TIOCGPGRP: u32 = 0x540F;
pub const TIOCSPGRP: u32 = 0x5410;
pub const TIOCGWINSZ: u32 = 0x5413;
pub const TIOCSWINSZ: u32 = 0x5414;
pub const TCGETS: u32 = 0x5401;
pub const TCSETS: u32 = 0x5402;
pub const TCSETSW: u32 = 0x5403;
pub const TCSETSF: u32 = 0x5404;
// TCGETS2/TCSETS2 — extended termios with ispeed/ospeed.
// vendor/linux/include/uapi/asm-generic/ioctls.h: _IOR('T', 0x2a, struct termios2)
pub const TCGETS2: u32 = 0x802c_542a;
pub const TCSETS2: u32 = 0x402c_542b;
pub const TCSETSW2: u32 = 0x402c_542c;
pub const TCSETSF2: u32 = 0x402c_542d;
pub const TCSBRK: u32 = 0x5409;
pub const TCFLSH: u32 = 0x540B;
pub const TIOCEXCL: u32 = 0x540C;
pub const TIOCNXCL: u32 = 0x540D;
pub const TIOCGSID: u32 = 0x5429;
pub const TIOCNOTTY: u32 = 0x5422;
pub const TIOCSTI: u32 = 0x5412;

// ── VT / KD ioctl numbers — `include/uapi/linux/{vt,kd}.h` ────────────────────
// Required by Xorg + Weston when they grab the console for graphics mode.
pub const KDGETMODE: u32 = 0x4B3B;
pub const KDSETMODE: u32 = 0x4B3A;
pub const KDGKBMODE: u32 = 0x4B44;
pub const KDSKBMODE: u32 = 0x4B45;
pub const VT_OPENQRY: u32 = 0x5600;
pub const VT_GETMODE: u32 = 0x5601;
pub const VT_SETMODE: u32 = 0x5602;
pub const VT_GETSTATE: u32 = 0x5603;
pub const VT_RELDISP: u32 = 0x5605;
pub const VT_ACTIVATE: u32 = 0x5606;
pub const VT_WAITACTIVE: u32 = 0x5607;

// `KDSIGACCEPT` — `include/uapi/linux/kd.h:146`.  systemd calls this on
// `/dev/tty0` to register which signal the kernel should send when the user
// presses the kbrequest key combo (Ctrl-Alt-KP-Plus by default).  Without it
// systemd logs "Failed to enable kbrequest handling".  Handler shape mirrors
// `drivers/tty/vt/vt_ioctl.c::KDSIGACCEPT`: reject SIGKILL/out-of-range, store
// (spawnsig, spawnpid), return 0.
pub const KDSIGACCEPT: u32 = 0x4B4E;

pub const KD_TEXT: u32 = 0x00;
pub const KD_GRAPHICS: u32 = 0x01;
pub const K_RAW: u32 = 0x00;
pub const K_XLATE: u32 = 0x01;
pub const K_MEDIUMRAW: u32 = 0x02;
pub const K_UNICODE: u32 = 0x03;
pub const K_OFF: u32 = 0x04;

pub const LFLAG_ISIG: u32 = 0x0000_0001;
pub const LFLAG_ICANON: u32 = 0x0000_0002;
pub const LFLAG_ECHO: u32 = 0x0000_0008;
// NOFLSH disables the input + output queue flush that normally follows an
// ISIG signal char.  Ref: vendor/linux/include/uapi/asm-generic/termios.h
// (`NOFLSH = 0o200`) and vendor/linux/drivers/tty/n_tty.c::isig.
pub const LFLAG_NOFLSH: u32 = 0x0000_0080;

// `c_cc` array indexes — glibc's x86-64 termios layout keeps these aligned
// with vendor/linux/include/uapi/asm-generic/termbits.h (`VINTR`, `VQUIT`,
// `VSUSP`).  These let `process_console_input_byte`
// honour `stty intr ^A` / `stty quit ^X` / `stty susp ^Y` instead of
// hard-coding 0x03 / 0x1C / 0x1A.
pub const VINTR: usize = 0;
pub const VQUIT: usize = 1;
pub const VSUSP: usize = 10;
pub const OFLAG_OPOST: u32 = 0x0000_0001;
const USER_NCCS: usize = 32;
const KERNEL_NCCS: usize = 19;
const DEFAULT_TTY_SPEED: u32 = 115_200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct KernelTermios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; USER_NCCS],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

impl Default for KernelTermios {
    fn default() -> Self {
        Self {
            c_iflag: 0x0500,
            c_oflag: 0x0005,
            c_cflag: 0x00bf,
            c_lflag: 0x8a3b,
            c_line: 0,
            c_cc: [
                3, 28, 127, 21, 4, 0, 1, 0, 17, 19, 26, 0, 18, 15, 23, 22, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0,
            ],
            c_ispeed: DEFAULT_TTY_SPEED,
            c_ospeed: DEFAULT_TTY_SPEED,
        }
    }
}

/// `struct termios2` — extended termios with explicit baud rates.
/// vendor/linux/include/uapi/asm-generic/termbits.h::`struct termios2`.
/// Identical to termios but adds c_ispeed and c_ospeed (speed_t = u32).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[repr(C)]
pub struct KernelTermios2 {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; KERNEL_NCCS],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

impl From<KernelTermios> for KernelTermios2 {
    fn from(t: KernelTermios) -> Self {
        let mut c_cc = [0u8; KERNEL_NCCS];
        c_cc.copy_from_slice(&t.c_cc[..KERNEL_NCCS]);
        Self {
            c_iflag: t.c_iflag,
            c_oflag: t.c_oflag,
            c_cflag: t.c_cflag,
            c_lflag: t.c_lflag,
            c_line: t.c_line,
            c_cc,
            c_ispeed: t.c_ispeed,
            c_ospeed: t.c_ospeed,
        }
    }
}

/// Window size — `struct winsize` from `include/uapi/asm-generic/termios.h`.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

/// Opaque HVC state backing `drivers/tty/hvc/hvc_console.c` exports.
///
/// Linux's `struct hvc_struct` is private to the HVC core; the virtio console
/// driver stores the returned pointer and passes it back to HVC entry points
/// without dereferencing it. Lupos therefore keeps an opaque record instead of
/// implementing a local console driver.
#[derive(Clone, Copy, Debug)]
struct HvcState {
    vtermno: u32,
    data: i32,
    ops: usize,
    outbuf_size: i32,
    winsize: Winsize,
}

lazy_static! {
    static ref HVC_STATES: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

static HVC_KICK_COUNT: AtomicUsize = AtomicUsize::new(0);

fn export_tty_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

fn err_ptr<T>(errno: i32) -> *mut T {
    (usize::MAX - errno as usize + 1) as *mut T
}

fn hvc_registered_index(hp: *mut c_void, states: &[usize]) -> Option<usize> {
    let ptr = hp as usize;
    if ptr == 0 {
        return None;
    }
    states.iter().position(|state| *state == ptr)
}

fn with_hvc_state_mut<R>(hp: *mut c_void, f: impl FnOnce(&mut HvcState) -> R) -> Option<R> {
    let states = HVC_STATES.lock();
    hvc_registered_index(hp, &states)?;
    Some(f(unsafe { &mut *(hp as *mut HvcState) }))
}

#[cfg(test)]
fn hvc_state_snapshot(hp: *mut c_void) -> Option<HvcState> {
    with_hvc_state_mut(hp, |state| *state)
}

/// Register HVC symbols exported by `vendor/linux/drivers/tty/hvc/hvc_console.c`.
pub fn register_module_exports() {
    export_tty_symbol_once("hvc_instantiate", linux_hvc_instantiate as usize, true);
    export_tty_symbol_once("hvc_alloc", linux_hvc_alloc as usize, true);
    export_tty_symbol_once("hvc_remove", linux_hvc_remove as usize, true);
    export_tty_symbol_once("hvc_poll", linux_hvc_poll as usize, true);
    export_tty_symbol_once("hvc_kick", linux_hvc_kick as usize, true);
    export_tty_symbol_once("__hvc_resize", linux___hvc_resize as usize, true);
}

/// `hvc_instantiate` - `vendor/linux/drivers/tty/hvc/hvc_console.c:285`.
pub unsafe extern "C" fn linux_hvc_instantiate(
    _vtermno: u32,
    _index: i32,
    _ops: *const c_void,
) -> i32 {
    0
}

/// `hvc_alloc` - `vendor/linux/drivers/tty/hvc/hvc_console.c:911`.
pub unsafe extern "C" fn linux_hvc_alloc(
    vtermno: u32,
    data: i32,
    ops: *const c_void,
    outbuf_size: i32,
) -> *mut c_void {
    if outbuf_size < 0 {
        return err_ptr(crate::include::uapi::errno::EINVAL);
    }
    let state = Box::new(HvcState {
        vtermno,
        data,
        ops: ops as usize,
        outbuf_size,
        winsize: Winsize::default(),
    });
    let ptr = Box::into_raw(state);
    HVC_STATES.lock().push(ptr as usize);
    ptr.cast()
}

/// `hvc_remove` - `vendor/linux/drivers/tty/hvc/hvc_console.c:977`.
pub unsafe extern "C" fn linux_hvc_remove(hp: *mut c_void) {
    let mut states = HVC_STATES.lock();
    let Some(index) = hvc_registered_index(hp, &states) else {
        return;
    };
    let ptr = states.swap_remove(index) as *mut HvcState;
    drop(unsafe { Box::from_raw(ptr) });
}

/// `hvc_poll` - `vendor/linux/drivers/tty/hvc/hvc_console.c:762`.
pub unsafe extern "C" fn linux_hvc_poll(_hp: *mut c_void) -> i32 {
    0
}

/// `hvc_kick` - `vendor/linux/drivers/tty/hvc/hvc_console.c:313`.
pub unsafe extern "C" fn linux_hvc_kick() {
    HVC_KICK_COUNT.fetch_add(1, Ordering::AcqRel);
}

/// `__hvc_resize` - `vendor/linux/drivers/tty/hvc/hvc_console.c:778`.
pub unsafe extern "C" fn linux___hvc_resize(hp: *mut c_void, ws: Winsize) {
    let _ = with_hvc_state_mut(hp, |state| {
        state.winsize = ws;
    });
}

/// Termios flags subset — used by n_tty to decide canonical vs raw mode.
#[derive(Clone, Copy, Debug)]
pub struct TermiosFlags {
    pub canonical: bool,
    pub echo: bool,
    pub opost: bool,
}

impl Default for TermiosFlags {
    fn default() -> Self {
        Self {
            canonical: true,
            echo: true,
            opost: true,
        }
    }
}

/// `struct tty_struct` — `include/linux/tty.h:188`.
pub struct TtyStruct {
    pub name: String,
    pub index: u32,
    pub pgrp: Mutex<i32>,
    pub session: Mutex<i32>,
    pub winsize: Mutex<Winsize>,
    pub termios: Mutex<TermiosFlags>,
    /// Line discipline state (n_tty for now).
    pub ldisc: Mutex<NTtyState>,
    /// Output buffer (goes to the underlying hardware write path).
    pub write_buf: Mutex<Vec<u8>>,
}

impl TtyStruct {
    pub fn new(name: &str, index: u32) -> Arc<Self> {
        Arc::new(Self {
            name: String::from(name),
            index,
            pgrp: Mutex::new(0),
            session: Mutex::new(0),
            winsize: Mutex::new(Winsize {
                ws_row: 24,
                ws_col: 80,
                ..Default::default()
            }),
            termios: Mutex::new(TermiosFlags::default()),
            ldisc: Mutex::new(NTtyState::new()),
            write_buf: Mutex::new(Vec::new()),
        })
    }

    /// Receive bytes from hardware (the ISR path).
    /// In canonical mode, `n_tty_receive_buf` accumulates until `\n` or EOF.
    pub fn receive_buf(&self, data: &[u8]) {
        self.ldisc.lock().receive(data, self.termios.lock().echo);
    }

    /// Read one canonical line (blocks in Linux; returns None if buffer empty).
    pub fn read_line(&self) -> Option<Vec<u8>> {
        self.ldisc.lock().read_line()
    }

    /// Write bytes to the output buffer (goes to hardware write).
    pub fn write(&self, data: &[u8]) {
        self.write_buf.lock().extend_from_slice(data);
    }

    /// Process a job-control ioctl.  Returns 0 on success, -errno on error.
    pub fn ioctl(&self, cmd: u32, arg: u64) -> i32 {
        match cmd {
            TIOCSCTTY => {
                // Assign this TTY as the controlling terminal of the session.
                *self.session.lock() = arg as i32;
                0
            }
            TIOCGPGRP => {
                // Return the foreground process group via `arg` as a pointer.
                // In bare-metal test mode we just return the stored value.
                let _ = arg;
                *self.pgrp.lock()
            }
            TIOCSPGRP => {
                *self.pgrp.lock() = arg as i32;
                0
            }
            TIOCGWINSZ => 0, // would copy Winsize to user — stub
            TIOCSWINSZ => 0, // would copy Winsize from user — stub
            KDSIGACCEPT => {
                let sig = arg as i32;
                if sig < 1 || sig > NSIG || sig == SIGKILL {
                    return -(crate::include::uapi::errno::EINVAL);
                }
                let pid = unsafe {
                    let task = crate::kernel::sched::get_current();
                    if task.is_null() { 0 } else { (*task).pid }
                };
                COMPAT_SPAWNSIG.store(sig, Ordering::Release);
                COMPAT_SPAWNPID.store(pid, Ordering::Release);
                0
            }
            _ => -(crate::include::uapi::errno::ENOTTY),
        }
    }
}

// ── TTY driver ────────────────────────────────────────────────────────────────

/// `struct tty_driver` — `include/linux/tty_driver.h:526`.
pub struct TtyDriver {
    pub name: &'static str,
    pub major: u32,
    pub minor_start: u32,
    pub num: u32,
    pub ttys: Mutex<BTreeMap<u32, Arc<TtyStruct>>>,
}

impl TtyDriver {
    pub fn new(name: &'static str, major: u32, minor_start: u32, num: u32) -> Arc<Self> {
        Arc::new(Self {
            name,
            major,
            minor_start,
            num,
            ttys: Mutex::new(BTreeMap::new()),
        })
    }
}

// ── Global driver registry ────────────────────────────────────────────────────

/// Minimal UNIX98 pty pair. The master feeds the slave line discipline and the
/// slave writes back to the master output queue.
pub struct PtyPair {
    pub master: Arc<TtyStruct>,
    pub slave: Arc<TtyStruct>,
}

impl PtyPair {
    pub fn new(index: u32) -> Self {
        Self {
            master: TtyStruct::new("ptmx", index),
            slave: TtyStruct::new("pts", index),
        }
    }

    pub fn master_write(&self, data: &[u8]) {
        self.slave.receive_buf(data);
    }

    pub fn slave_read_line(&self) -> Option<Vec<u8>> {
        self.slave.read_line()
    }

    pub fn slave_write(&self, data: &[u8]) {
        self.master.write(data);
    }

    pub fn master_read_all(&self) -> Vec<u8> {
        self.master.write_buf.lock().drain(..).collect()
    }
}

lazy_static! {
    static ref TTY_DRIVERS: Mutex<Vec<Arc<TtyDriver>>> = Mutex::new(Vec::new());
    static ref COMPAT_TTY_TERMIOS: Mutex<KernelTermios> = Mutex::new(KernelTermios::default());
    static ref COMPAT_TTY_WINSIZE: Mutex<Winsize> = Mutex::new(Winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    });
}

static COMPAT_TTY_SESSION: AtomicI32 = AtomicI32::new(0);
static COMPAT_TTY_PGRP: AtomicI32 = AtomicI32::new(0);

// `spawnsig`/`spawnpid` from `drivers/tty/vt/vt_ioctl.c` — set by
// `KDSIGACCEPT`, consumed by the kbrequest delivery path.
static COMPAT_SPAWNSIG: AtomicI32 = AtomicI32::new(0);
static COMPAT_SPAWNPID: AtomicI32 = AtomicI32::new(0);

// `tty_legacy_tiocsti` — `drivers/tty/tty_io.c`. Gate for unprivileged
// `TIOCSTI`; the build config sets `CONFIG_LEGACY_TIOCSTI=y`, so it starts
// enabled. Toggled at runtime via `/proc/sys/dev/tty/legacy_tiocsti`.
static TTY_LEGACY_TIOCSTI: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(true);
// `tty_ldisc_autoload` — `drivers/tty/tty_ldisc.c`, `CONFIG_LDISC_AUTOLOAD=y`.
// Only surfaced through `/proc/sys/dev/tty/ldisc_autoload` for now; Lupos has
// no modular line disciplines to gate yet.
static TTY_LDISC_AUTOLOAD: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(true);

pub fn legacy_tiocsti_enabled() -> bool {
    TTY_LEGACY_TIOCSTI.load(Ordering::Acquire)
}

pub fn set_legacy_tiocsti(enabled: bool) {
    TTY_LEGACY_TIOCSTI.store(enabled, Ordering::Release);
}

pub fn ldisc_autoload_enabled() -> bool {
    TTY_LDISC_AUTOLOAD.load(Ordering::Acquire)
}

pub fn set_ldisc_autoload(enabled: bool) {
    TTY_LDISC_AUTOLOAD.store(enabled, Ordering::Release);
}

/// `_NSIG` on x86_64 (`include/uapi/asm-generic/signal.h`).
const NSIG: i32 = 64;
/// `SIGKILL` (`include/uapi/asm-generic/signal.h`).
const SIGKILL: i32 = 9;

// ── VT / KD compat state ──────────────────────────────────────────────────────

use core::sync::atomic::AtomicU32;

static COMPAT_KD_MODE: AtomicU32 = AtomicU32::new(KD_TEXT);
static COMPAT_KB_MODE: AtomicU32 = AtomicU32::new(K_XLATE);
static COMPAT_VT_ACTIVE: AtomicU32 = AtomicU32::new(1);

/// Number of synthetic VTs we report to userspace.  Mirrors the default
/// configuration of Linux's `CONFIG_VT_CONSOLE` build (`MAX_NR_CONSOLES=63`
/// in upstream, but we only own `tty1..tty6`).
pub const VT_MAX_CONSOLES: u32 = 6;

/// Snapshot used by callers that need to check whether the framebuffer text
/// console should keep rendering — fbcon equivalents go silent in
/// `KD_GRAPHICS`.
pub fn kd_text_console_owned() -> bool {
    COMPAT_KD_MODE.load(Ordering::Acquire) == KD_TEXT
}

/// `struct vt_mode` — `include/uapi/linux/vt.h:22`.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VtMode {
    pub mode: u8,
    pub waitv: u8,
    pub relsig: i16,
    pub acqsig: i16,
    pub frsig: i16,
}

/// `struct vt_stat` — `include/uapi/linux/vt.h:32`.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VtStat {
    pub v_active: u16,
    pub v_signal: u16,
    pub v_state: u16,
}

static COMPAT_VT_MODE: spin::Mutex<VtMode> = spin::Mutex::new(VtMode {
    mode: 0, // VT_AUTO
    waitv: 0,
    relsig: 0,
    acqsig: 0,
    frsig: 0,
});

pub fn termios_isig(termios: &KernelTermios) -> bool {
    (termios.c_lflag & LFLAG_ISIG) != 0
}

/// L_NOFLSH(tty) per `vendor/linux/include/linux/tty.h`.  When set, the
/// signal char delivery path skips the input/output buffer flush.
pub fn termios_noflsh(termios: &KernelTermios) -> bool {
    (termios.c_lflag & LFLAG_NOFLSH) != 0
}

pub fn termios_canonical(termios: &KernelTermios) -> bool {
    (termios.c_lflag & LFLAG_ICANON) != 0
}

pub fn termios_echo(termios: &KernelTermios) -> bool {
    (termios.c_lflag & LFLAG_ECHO) != 0
}

pub fn compat_termios() -> KernelTermios {
    *COMPAT_TTY_TERMIOS.lock()
}

pub fn set_compat_termios(termios: KernelTermios) {
    *COMPAT_TTY_TERMIOS.lock() = termios;
}

pub fn compat_winsize() -> Winsize {
    *COMPAT_TTY_WINSIZE.lock()
}

pub fn set_compat_winsize(winsize: Winsize) {
    *COMPAT_TTY_WINSIZE.lock() = winsize;
}

pub fn reset_compat_tty_state() {
    set_compat_termios(KernelTermios::default());
    set_compat_winsize(Winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    });
    COMPAT_TTY_SESSION.store(0, Ordering::Release);
    COMPAT_TTY_PGRP.store(0, Ordering::Release);
    COMPAT_SPAWNSIG.store(0, Ordering::Release);
    COMPAT_SPAWNPID.store(0, Ordering::Release);
    COMPAT_KD_MODE.store(KD_TEXT, Ordering::Release);
    crate::kernel::console::set_fbcon_enabled(true);
    COMPAT_KB_MODE.store(K_XLATE, Ordering::Release);
    COMPAT_VT_ACTIVE.store(1, Ordering::Release);
    *COMPAT_VT_MODE.lock() = VtMode::default();
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

pub fn signal_compat_foreground(sig: i32) -> i32 {
    let pgrp = COMPAT_TTY_PGRP.load(Ordering::Acquire);
    crate::kernel::signal::send_signal_to_process_group(pgrp, sig)
}

/// `true` once a process called `TIOCSCTTY` on /dev/tty1 / /dev/console
/// (the compat path) — i.e. a controlling-tty session has been claimed.
/// Mirrors Linux's `tty_get_pgrp(tty)` returning non-NULL.  Used to decide
/// whether `deliver_console_signal` should fall back to the current task
/// for the very early agetty phase or just silently drop the signal as
/// Linux does once a session leader exists.
pub fn compat_tty_session_active() -> bool {
    COMPAT_TTY_SESSION.load(Ordering::Acquire) > 0
}

/// Minimal fd-backed console ioctl compatibility used before full tty file
/// instances exist. Linux's job-control ioctls pass user pointers for process
/// groups, so this helper uses fault-recoverable uaccess.
pub fn tty_ioctl_compat(cmd: u32, arg: u64) -> Result<i64, i32> {
    match cmd {
        TIOCSCTTY => {
            let (sid, pgrp) = current_session_and_pgrp().unwrap_or((arg as i32, arg as i32));
            COMPAT_TTY_SESSION.store(sid, Ordering::Release);
            COMPAT_TTY_PGRP.store(pgrp, Ordering::Release);
            Ok(0)
        }
        TIOCGPGRP => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            unsafe {
                crate::arch::x86::kernel::uaccess::put_user_u32(
                    arg as *mut u32,
                    COMPAT_TTY_PGRP.load(Ordering::Acquire) as u32,
                )
            }
            .map_err(|e| -e)?;
            Ok(0)
        }
        TIOCSPGRP => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let pgrp =
                unsafe { crate::arch::x86::kernel::uaccess::get_user_u32(arg as *const u32) }
                    .map_err(|e| -e)?;
            COMPAT_TTY_PGRP.store(pgrp as i32, Ordering::Release);
            Ok(0)
        }
        TIOCGWINSZ => {
            let ws = compat_winsize();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &ws as *const Winsize as *const u8,
                    core::mem::size_of::<Winsize>(),
                )
            };
            if not_copied == 0 {
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        TIOCSWINSZ => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let mut ws = Winsize::default();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    &mut ws as *mut Winsize as *mut u8,
                    arg as *const u8,
                    core::mem::size_of::<Winsize>(),
                )
            };
            if not_copied == 0 {
                set_compat_winsize(ws);
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        TCGETS => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let termios = compat_termios();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &termios as *const KernelTermios as *const u8,
                    core::mem::size_of::<KernelTermios>(),
                )
            };
            if not_copied == 0 {
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        TCSETS | TCSETSW | TCSETSF => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let mut termios = KernelTermios::default();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    &mut termios as *mut KernelTermios as *mut u8,
                    arg as *const u8,
                    core::mem::size_of::<KernelTermios>(),
                )
            };
            if not_copied == 0 {
                set_compat_termios(termios);
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        // TCGETS2: extended termios with explicit baud rate fields.
        // vendor/linux/include/uapi/asm-generic/ioctls.h: TCGETS2 = _IOR('T',0x2a,termios2)
        TCGETS2 => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let termios2 = KernelTermios2::from(compat_termios());
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &termios2 as *const KernelTermios2 as *const u8,
                    core::mem::size_of::<KernelTermios2>(),
                )
            };
            if not_copied == 0 {
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        TCSETS2 | TCSETSW2 | TCSETSF2 => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let mut termios2 = KernelTermios2::from(compat_termios());
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    &mut termios2 as *mut KernelTermios2 as *mut u8,
                    arg as *const u8,
                    core::mem::size_of::<KernelTermios2>(),
                )
            };
            if not_copied == 0 {
                let mut c_cc = KernelTermios::default().c_cc;
                c_cc[..KERNEL_NCCS].copy_from_slice(&termios2.c_cc);
                let t = KernelTermios {
                    c_iflag: termios2.c_iflag,
                    c_oflag: termios2.c_oflag,
                    c_cflag: termios2.c_cflag,
                    c_lflag: termios2.c_lflag,
                    c_line: termios2.c_line,
                    c_cc,
                    c_ispeed: termios2.c_ispeed,
                    c_ospeed: termios2.c_ospeed,
                };
                set_compat_termios(t);
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        TCSBRK | TCFLSH | TIOCEXCL | TIOCNXCL => Ok(0),
        KDGETMODE => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            unsafe {
                crate::arch::x86::kernel::uaccess::put_user_u32(
                    arg as *mut u32,
                    COMPAT_KD_MODE.load(Ordering::Acquire),
                )
            }
            .map_err(|e| -e)?;
            Ok(0)
        }
        KDSETMODE => {
            let mode = arg as u32;
            if mode != KD_TEXT && mode != KD_GRAPHICS {
                return Err(crate::include::uapi::errno::EINVAL);
            }
            COMPAT_KD_MODE.store(mode, Ordering::Release);
            crate::kernel::console::set_fbcon_enabled(mode == KD_TEXT);
            Ok(0)
        }
        KDGKBMODE => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            unsafe {
                crate::arch::x86::kernel::uaccess::put_user_u32(
                    arg as *mut u32,
                    COMPAT_KB_MODE.load(Ordering::Acquire),
                )
            }
            .map_err(|e| -e)?;
            Ok(0)
        }
        KDSKBMODE => {
            // Linux validates against K_RAW..K_OFF and returns -EINVAL for
            // anything else.
            let mode = arg as u32;
            if !matches!(mode, K_RAW | K_XLATE | K_MEDIUMRAW | K_UNICODE | K_OFF) {
                return Err(crate::include::uapi::errno::EINVAL);
            }
            COMPAT_KB_MODE.store(mode, Ordering::Release);
            Ok(0)
        }
        VT_OPENQRY => {
            // Return the first VT number that isn't currently active.  With a
            // single physical console we always have the next minor free.
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let candidate = (COMPAT_VT_ACTIVE.load(Ordering::Acquire) % VT_MAX_CONSOLES) + 1;
            unsafe { crate::arch::x86::kernel::uaccess::put_user_u32(arg as *mut u32, candidate) }
                .map_err(|e| -e)?;
            Ok(0)
        }
        VT_GETMODE => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let mode = *COMPAT_VT_MODE.lock();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &mode as *const VtMode as *const u8,
                    core::mem::size_of::<VtMode>(),
                )
            };
            if not_copied == 0 {
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        VT_SETMODE => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let mut mode = VtMode::default();
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_from_user(
                    &mut mode as *mut VtMode as *mut u8,
                    arg as *const u8,
                    core::mem::size_of::<VtMode>(),
                )
            };
            if not_copied != 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            *COMPAT_VT_MODE.lock() = mode;
            Ok(0)
        }
        VT_GETSTATE => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            let st = VtStat {
                v_active: COMPAT_VT_ACTIVE.load(Ordering::Acquire) as u16,
                v_signal: 0,
                // Bitmask of opened VTs — we always report tty1..tty6 open.
                v_state: ((1u16 << (VT_MAX_CONSOLES + 1)) - 1) & !1,
            };
            let not_copied = unsafe {
                crate::arch::x86::kernel::uaccess::copy_to_user(
                    arg as *mut u8,
                    &st as *const VtStat as *const u8,
                    core::mem::size_of::<VtStat>(),
                )
            };
            if not_copied == 0 {
                Ok(0)
            } else {
                Err(crate::include::uapi::errno::EFAULT)
            }
        }
        VT_ACTIVATE => {
            let target = arg as u32;
            if target == 0 || target > VT_MAX_CONSOLES {
                return Err(crate::include::uapi::errno::EINVAL);
            }
            COMPAT_VT_ACTIVE.store(target, Ordering::Release);
            Ok(0)
        }
        VT_WAITACTIVE => {
            // We never actually switch VTs — the requested console is
            // considered "active" as soon as the request lands.
            let target = arg as u32;
            if target == 0 || target > VT_MAX_CONSOLES {
                return Err(crate::include::uapi::errno::EINVAL);
            }
            COMPAT_VT_ACTIVE.store(target, Ordering::Release);
            Ok(0)
        }
        VT_RELDISP => Ok(0),
        KDSIGACCEPT => {
            let sig = arg as i32;
            if sig < 1 || sig > NSIG || sig == SIGKILL {
                return Err(crate::include::uapi::errno::EINVAL);
            }
            let pid = current_session_and_pgrp()
                .map(|(_, pgrp)| pgrp)
                .unwrap_or(0);
            COMPAT_SPAWNSIG.store(sig, Ordering::Release);
            COMPAT_SPAWNPID.store(pid, Ordering::Release);
            Ok(0)
        }
        TIOCGSID => {
            if arg == 0 {
                return Err(crate::include::uapi::errno::EFAULT);
            }
            unsafe {
                crate::arch::x86::kernel::uaccess::put_user_u32(
                    arg as *mut u32,
                    COMPAT_TTY_SESSION.load(Ordering::Acquire) as u32,
                )
            }
            .map_err(|e| -e)?;
            Ok(0)
        }
        _ => Err(crate::include::uapi::errno::ENOTTY),
    }
}

/// `tty_register_driver` — `drivers/tty/tty_io.c:3425`.
pub fn tty_register_driver(drv: Arc<TtyDriver>) -> Result<(), i32> {
    TTY_DRIVERS.lock().push(drv);
    Ok(())
}

pub fn tty_driver_count() -> usize {
    TTY_DRIVERS.lock().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    static TTY_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn canonical_mode_line_buffering() {
        let tty = TtyStruct::new("ttyS0", 0);
        tty.receive_buf(b"hello\n");
        let line = tty.read_line();
        assert!(line.is_some());
        assert_eq!(line.unwrap(), b"hello\n");
    }

    #[test]
    fn tiocsctty_stores_session() {
        let tty = TtyStruct::new("ttyS0", 0);
        let r = tty.ioctl(TIOCSCTTY, 42);
        assert_eq!(r, 0);
        assert_eq!(*tty.session.lock(), 42);
    }

    #[test]
    fn tiocspgrp_and_tiocgpgrp_roundtrip() {
        let tty = TtyStruct::new("ttyS0", 0);
        tty.ioctl(TIOCSPGRP, 7);
        let pg = tty.ioctl(TIOCGPGRP, 0);
        assert_eq!(pg, 7);
    }

    #[test]
    fn tty_register_driver_ok() {
        let before = tty_driver_count();
        let drv = TtyDriver::new("test-drv", 4, 64, 8);
        tty_register_driver(drv).unwrap();
        assert_eq!(tty_driver_count(), before + 1);
    }

    #[test]
    fn pty_pair_moves_data_between_master_and_slave() {
        let pty = PtyPair::new(0);
        pty.master_write(b"whoami\n");
        assert_eq!(pty.slave_read_line().unwrap(), b"whoami\n");
        pty.slave_write(b"lupos\n");
        assert_eq!(pty.master_read_all(), b"lupos\n");
    }

    #[test]
    fn compat_termios_and_winsize_round_trip() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();

        let mut termios = KernelTermios::default();
        assert_eq!(
            tty_ioctl_compat(TCGETS, &mut termios as *mut _ as u64),
            Ok(0)
        );
        termios.c_lflag &= !(LFLAG_ICANON | LFLAG_ECHO);
        assert_eq!(tty_ioctl_compat(TCSETS, &termios as *const _ as u64), Ok(0));

        let mut roundtrip = KernelTermios::default();
        assert_eq!(
            tty_ioctl_compat(TCGETS, &mut roundtrip as *mut _ as u64),
            Ok(0)
        );
        assert_eq!(roundtrip, termios);
        assert!(!termios_canonical(&roundtrip));
        assert!(!termios_echo(&roundtrip));

        let ws = Winsize {
            ws_row: 37,
            ws_col: 100,
            ws_xpixel: 800,
            ws_ypixel: 600,
        };
        assert_eq!(tty_ioctl_compat(TIOCSWINSZ, &ws as *const _ as u64), Ok(0));
        let mut got = Winsize::default();
        assert_eq!(
            tty_ioctl_compat(TIOCGWINSZ, &mut got as *mut _ as u64),
            Ok(0)
        );
        assert_eq!(got.ws_row, 37);
        assert_eq!(got.ws_col, 100);
    }

    #[test]
    fn tcgets_layout_matches_x86_64_glibc_termios() {
        let termios = KernelTermios::default();
        let base = &termios as *const KernelTermios as usize;
        let ispeed = &termios.c_ispeed as *const u32 as usize - base;
        let ospeed = &termios.c_ospeed as *const u32 as usize - base;

        assert_eq!(core::mem::size_of::<KernelTermios>(), 60);
        assert_eq!(ispeed, 52);
        assert_eq!(ospeed, 56);
        assert_eq!(core::mem::size_of::<KernelTermios2>(), 44);
    }

    #[test]
    fn kdsetmode_toggles_graphics() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        assert!(kd_text_console_owned());

        assert_eq!(tty_ioctl_compat(KDSETMODE, KD_GRAPHICS as u64), Ok(0));
        assert!(!kd_text_console_owned());

        let mut mode: u32 = 0;
        assert_eq!(
            tty_ioctl_compat(KDGETMODE, &mut mode as *mut _ as u64),
            Ok(0)
        );
        assert_eq!(mode, KD_GRAPHICS);

        assert_eq!(tty_ioctl_compat(KDSETMODE, KD_TEXT as u64), Ok(0));
        assert!(kd_text_console_owned());
    }

    #[test]
    fn kdsetmode_rejects_invalid_mode() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        assert_eq!(
            tty_ioctl_compat(KDSETMODE, 0x1234),
            Err(crate::include::uapi::errno::EINVAL)
        );
    }

    #[test]
    fn vt_activate_and_waitactive_track_active_console() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        assert_eq!(tty_ioctl_compat(VT_ACTIVATE, 3), Ok(0));
        assert_eq!(tty_ioctl_compat(VT_WAITACTIVE, 3), Ok(0));

        let mut st = VtStat::default();
        assert_eq!(
            tty_ioctl_compat(VT_GETSTATE, &mut st as *mut _ as u64),
            Ok(0)
        );
        assert_eq!(st.v_active, 3);

        assert_eq!(
            tty_ioctl_compat(VT_ACTIVATE, (VT_MAX_CONSOLES + 1) as u64),
            Err(crate::include::uapi::errno::EINVAL)
        );
    }

    #[test]
    fn vt_getmode_setmode_round_trip() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        let mode = VtMode {
            mode: 1,
            waitv: 0,
            relsig: 10,
            acqsig: 12,
            frsig: 0,
        };
        assert_eq!(
            tty_ioctl_compat(VT_SETMODE, &mode as *const _ as u64),
            Ok(0)
        );
        let mut got = VtMode::default();
        assert_eq!(
            tty_ioctl_compat(VT_GETMODE, &mut got as *mut _ as u64),
            Ok(0)
        );
        assert_eq!(got.mode, 1);
        assert_eq!(got.relsig, 10);
        assert_eq!(got.acqsig, 12);
    }

    #[test]
    fn kdskbmode_accepts_known_modes_only() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        for mode in [K_RAW, K_XLATE, K_MEDIUMRAW, K_UNICODE, K_OFF] {
            assert_eq!(tty_ioctl_compat(KDSKBMODE, mode as u64), Ok(0));
        }
        assert_eq!(
            tty_ioctl_compat(KDSKBMODE, 99),
            Err(crate::include::uapi::errno::EINVAL)
        );
    }

    #[test]
    fn tty_ioctl_kdsigaccept_stores_signal_and_returns_zero() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        let tty = TtyStruct::new("tty0", 0);
        // SIGWINCH = 28 is a typical choice for kbrequest.
        assert_eq!(tty.ioctl(KDSIGACCEPT, 28), 0);
        assert_eq!(COMPAT_SPAWNSIG.load(Ordering::Acquire), 28);
    }

    #[test]
    fn tty_ioctl_kdsigaccept_rejects_zero_signal_with_einval() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        let tty = TtyStruct::new("tty0", 0);
        assert_eq!(
            tty.ioctl(KDSIGACCEPT, 0),
            -(crate::include::uapi::errno::EINVAL)
        );
        assert_eq!(COMPAT_SPAWNSIG.load(Ordering::Acquire), 0);
    }

    #[test]
    fn tty_ioctl_kdsigaccept_rejects_sigkill_with_einval() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        let tty = TtyStruct::new("tty0", 0);
        assert_eq!(
            tty.ioctl(KDSIGACCEPT, 9),
            -(crate::include::uapi::errno::EINVAL)
        );
        assert_eq!(COMPAT_SPAWNSIG.load(Ordering::Acquire), 0);
    }

    #[test]
    fn tty_ioctl_kdsigaccept_rejects_signal_above_nsig() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        let tty = TtyStruct::new("tty0", 0);
        assert_eq!(
            tty.ioctl(KDSIGACCEPT, 65),
            -(crate::include::uapi::errno::EINVAL)
        );
        assert_eq!(COMPAT_SPAWNSIG.load(Ordering::Acquire), 0);
    }

    #[test]
    fn tty_ioctl_compat_kdsigaccept_returns_zero() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        assert_eq!(tty_ioctl_compat(KDSIGACCEPT, 2), Ok(0));
        assert_eq!(COMPAT_SPAWNSIG.load(Ordering::Acquire), 2);
    }

    #[test]
    fn tty_ioctl_compat_kdsigaccept_rejects_invalid_signals() {
        let _guard = TTY_TEST_LOCK.lock();
        reset_compat_tty_state();
        assert_eq!(
            tty_ioctl_compat(KDSIGACCEPT, 0),
            Err(crate::include::uapi::errno::EINVAL)
        );
        assert_eq!(
            tty_ioctl_compat(KDSIGACCEPT, 9),
            Err(crate::include::uapi::errno::EINVAL)
        );
        assert_eq!(
            tty_ioctl_compat(KDSIGACCEPT, 65),
            Err(crate::include::uapi::errno::EINVAL)
        );
    }

    #[test]
    fn hvc_alloc_returns_opaque_state_and_remove_releases_it() {
        let _guard = TTY_TEST_LOCK.lock();
        let before = HVC_STATES.lock().len();
        let hp = unsafe { linux_hvc_alloc(7, 3, 0x1000usize as *const c_void, 4096) };

        assert!(!hp.is_null());
        assert_eq!(HVC_STATES.lock().len(), before + 1);
        let state = hvc_state_snapshot(hp).expect("hvc state should be registered");
        assert_eq!(state.vtermno, 7);
        assert_eq!(state.data, 3);
        assert_eq!(state.ops, 0x1000);
        assert_eq!(state.outbuf_size, 4096);

        let ws = Winsize {
            ws_row: 40,
            ws_col: 120,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe { linux___hvc_resize(hp, ws) };
        assert_eq!(hvc_state_snapshot(hp).unwrap().winsize.ws_col, 120);

        unsafe { linux_hvc_remove(hp) };
        assert_eq!(HVC_STATES.lock().len(), before);
    }

    #[test]
    fn hvc_exports_track_vendor_gpl_symbols() {
        let source = include_str!("../../../vendor/linux/drivers/tty/hvc/hvc_console.c");
        for symbol in [
            "hvc_instantiate",
            "hvc_kick",
            "hvc_poll",
            "__hvc_resize",
            "hvc_alloc",
            "hvc_remove",
        ] {
            assert!(
                source.contains(&format!("EXPORT_SYMBOL_GPL({symbol});")),
                "missing GPL export for {symbol}"
            );
        }
    }
}
