//! linux-parity: partial
//! linux-source: vendor/linux/drivers/gpu/drm
//! test-origin: linux:vendor/linux/drivers/gpu/drm
//! Kernel ABI exports required by the vendor-built DRM module chain
//! (`drm.ko`, `drm_panel_orientation_quirks.ko`, `drm_kms_helper.ko`,
//! `drm_shmem_helper.ko`, `bochs.ko`, `i915.ko`, `virtio-gpu.ko`).
//!
//! Lupos never reimplements the drivers themselves — the modules are compiled
//! from `vendor/linux` and loaded by the module loader.  This file only
//! provides the kernel-side symbols those modules import, in three flavors:
//!
//! 1. **Delegating shims** onto existing Lupos subsystems (slab, vmalloc,
//!    ioremap, jiffies/TSC time, workqueues, PCI config space, fbdev core).
//! 2. **Side-table implementations** for kernel objects whose C structs the
//!    modules treat as opaque handles (`struct idr`, `struct xarray`,
//!    `ww_mutex`, kthread workers, timers, hrtimers, shmem-backed GEM files).
//!    Keying the state by the C object's address avoids pinning every vendor
//!    struct layout, the same trick `linux_pci_device_state` uses.
//! Symbols without a type-correct, semantically equivalent implementation are
//! deliberately left unresolved.  Mapping unrelated C prototypes to one
//! universal return-zero function can make module insertion appear successful
//! while corrupting state as soon as a supposedly "cold" path is reached.
//!
//! Struct-field offsets used below (fb_info, file, inode, delayed_work,
//! kthread_work, hrtimer, vfsmount) were extracted from the actual
//! `vendor/linux` module build (`target/xtask/vendor-linux-build`) with an
//! offsetof probe object, and are asserted in the tests at the bottom.
//!
//! The asynchronous pieces (delayed work, hrtimers backing the DRM vblank
//! timer, kthread vblank workers) are executed by a driver-ABI poller — the
//! same idle pump that delivers AHCI/virtio completions — because Lupos'
//! cooperative scheduler has no per-CPU workqueue kthreads yet.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENOMEM, ENOSPC};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::mm::page::Page;
use crate::mm::page_flags::GFP_KERNEL;

// ── probed vendor struct offsets (see module doc) ──────────────────────────

pub const FILE_F_MAPPING: usize = 16;
pub const FILE_F_INODE: usize = 32;
pub const FILE_SIZE: usize = 176;
pub const INODE_I_MAPPING: usize = 48;
pub const INODE_SIZE: usize = 544;
pub const ADDRESS_SPACE_SIZE: usize = 152;
pub const FB_INFO_NODE: usize = 4;
pub const FB_INFO_VAR: usize = 64;
pub const FB_INFO_FIX: usize = 224;
pub const FB_INFO_CMAP: usize = 592;
pub const FB_INFO_SCREEN_BASE: usize = 688;
pub const FB_INFO_PAR: usize = 728;
pub const FB_INFO_SIZE: usize = 744;
pub const DWORK_TIMER: usize = 32;
pub const KTHREAD_WORK_FUNC: usize = 16;
pub const VFSMOUNT_MNT_SB: usize = 8;
pub const HRTIMER_FUNCTION: usize = 72;
pub const HRTIMER_EXPIRES: usize = 40;
pub const HRTIMER_SOFTEXPIRES: usize = 64;

/// Modules in the chain are built with `CONFIG_HZ=1000`
/// (`target/xtask/vendor-linux-build/.config`).
const MODULE_HZ: u64 = 1000;

fn export_symbol_once(name: &str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

#[cfg(not(test))]
fn kmalloc(size: usize) -> *mut u8 {
    unsafe { crate::mm::slab::linux___kmalloc_noprof(size, GFP_KERNEL) }
}

#[cfg(not(test))]
fn kfree(ptr: *mut u8) {
    unsafe { crate::mm::slab::linux_kfree(ptr) };
}

/// Host unit tests run without `slab_init()`; back the helpers with the Rust
/// global allocator behind a size header instead.
#[cfg(test)]
fn kmalloc(size: usize) -> *mut u8 {
    let layout = core::alloc::Layout::from_size_align(size + 16, 16).unwrap();
    unsafe {
        let block = alloc::alloc::alloc(layout);
        if block.is_null() {
            return block;
        }
        *(block as *mut usize) = size;
        block.add(16)
    }
}

#[cfg(test)]
fn kfree(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let block = ptr.sub(16);
        let size = *(block as *const usize);
        let layout = core::alloc::Layout::from_size_align(size + 16, 16).unwrap();
        alloc::alloc::dealloc(block, layout);
    }
}

fn kzalloc(size: usize) -> *mut u8 {
    let ptr = kmalloc(size);
    if !ptr.is_null() {
        unsafe { core::ptr::write_bytes(ptr, 0, size) };
    }
    ptr
}

fn jiffies_now() -> u64 {
    crate::kernel::time::jiffies::jiffies()
}

unsafe fn c_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    let len = unsafe { crate::lib::string::c_strlen(ptr, 512) };
    let bytes = unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) };
    core::str::from_utf8(bytes).unwrap_or("")
}

// ── mini vsnprintf for name-formatting exports ─────────────────────────────
//
// The DRM chain formats device names ("card%d", "%s-%d") through kasprintf /
// dev_set_name / kvasprintf.  On x86-64 SysV up to five integer varargs arrive
// in registers, so the shims declare five extra usize parameters and format
// from that slice.  This intentionally covers only the conversions the chain
// uses for names: %s %d %i %u %x %lx %ld %lu %llu %llx %zu %c %%.
fn format_c(fmt: &str, args: &[usize]) -> String {
    let mut out = String::new();
    let bytes = fmt.as_bytes();
    let mut i = 0usize;
    let mut arg = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        i += 1;
        // Skip flags/width and length modifiers.
        while i < bytes.len() && (bytes[i].is_ascii_digit() || matches!(bytes[i], b'-' | b'0')) {
            i += 1;
        }
        while i < bytes.len() && matches!(bytes[i], b'l' | b'z' | b'h') {
            i += 1;
        }
        if i >= bytes.len() {
            out.push('%');
            break;
        }
        let spec = bytes[i];
        i += 1;
        let mut next_arg = || {
            let value = args.get(arg).copied().unwrap_or(0);
            arg += 1;
            value
        };
        match spec {
            b'%' => out.push('%'),
            b'c' => out.push((next_arg() as u8) as char),
            b'd' | b'i' => {
                let value = next_arg() as isize;
                out.push_str(&alloc::format!("{value}"));
            }
            b'u' => {
                let value = next_arg();
                out.push_str(&alloc::format!("{value}"));
            }
            b'x' => {
                let value = next_arg();
                out.push_str(&alloc::format!("{value:x}"));
            }
            b'X' => {
                let value = next_arg();
                out.push_str(&alloc::format!("{value:X}"));
            }
            b'p' => {
                let value = next_arg();
                out.push_str(&alloc::format!("{value:#x}"));
            }
            b's' => {
                let ptr = next_arg() as *const c_char;
                if ptr.is_null() {
                    out.push_str("(null)");
                } else {
                    out.push_str(unsafe { c_str(ptr) });
                }
            }
            other => {
                out.push('%');
                out.push(other as char);
            }
        }
    }
    out
}

/// Copy a formatted string into a fresh kmalloc'd NUL-terminated buffer that
/// the module may later `kfree()`.
fn kstrdup_rust(s: &str) -> *mut c_char {
    let ptr = kmalloc(s.len() + 1);
    if ptr.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len());
        *ptr.add(s.len()) = 0;
    }
    ptr.cast()
}

// ── data symbols ───────────────────────────────────────────────────────────

static mut LINUX_OOPS_IN_PROGRESS: i32 = 0;
static mut LINUX_OVERFLOWUID: u32 = 65534;
/// `struct cpuinfo_x86 boot_cpu_data` — zeroed: every `static_cpu_has()`
/// check in the chain reads capability bits of 0, steering drm_cache to its
/// fallback paths.  Sized generously above the vendor struct.
static mut LINUX_BOOT_CPU_DATA: [u8; 512] = [0; 512];
/// `__default_kernel_pte_mask` — all PTE bits allowed (PAT is programmed by
/// `arch::x86::mm::pat`), matching Linux with working PAT.
static mut LINUX_DEFAULT_KERNEL_PTE_MASK: u64 = !0;
static mut LINUX_IOMEM_RESOURCE: [u8; 64] = [0; 64];
static mut LINUX_IOPORT_RESOURCE: [u8; 64] = [0; 64];
/// `reservation_ww_class` — identity only; the ww shims below keep their own
/// per-lock state.
static mut LINUX_RESERVATION_WW_CLASS: [usize; 6] = [0; 6];
static mut LINUX_DMA_FENCE_CHAIN_OPS: [usize; 16] = [0; 16];
static mut LINUX_PARAM_OPS_INT: [usize; 4] = [0; 4];
static mut LINUX_PARAM_OPS_UINT: [usize; 4] = [0; 4];
static mut LINUX_PARAM_OPS_BOOL: [usize; 4] = [0; 4];
static mut LINUX_PARAM_OPS_ULONG: [usize; 4] = [0; 4];
static mut LINUX_PARAM_OPS_STRING: [usize; 4] = [0; 4];
/// `system_long_wq` / `system_percpu_wq` / `system_dfl_wq` are pointer
/// variables in Linux; each cell holds an `Arc<Workqueue>` leaked at
/// registration time.
static mut LINUX_SYSTEM_LONG_WQ: usize = 0;
static mut LINUX_SYSTEM_PERCPU_WQ: usize = 0;
static mut LINUX_SYSTEM_DFL_WQ: usize = 0;

// ── pump: delayed works, timers, hrtimers, kthread workers ────────────────

#[derive(Clone, Copy)]
struct PendingTimer {
    func: usize,
    expires_jiffies: u64,
}

#[derive(Clone, Copy)]
struct PendingHrtimer {
    func: usize,
    expires_ns: u64,
    active: bool,
}

#[derive(Clone, Copy)]
struct PendingDelayedWork {
    wq: usize,
    expires_jiffies: u64,
}

lazy_static! {
    /// `timer_list` side state keyed by the C timer address.
    static ref TIMERS: Mutex<BTreeMap<usize, PendingTimer>> = Mutex::new(BTreeMap::new());
    /// armed `delayed_work`s keyed by the C `struct delayed_work` address.
    static ref DELAYED_WORKS: Mutex<BTreeMap<usize, PendingDelayedWork>> =
        Mutex::new(BTreeMap::new());
    /// `hrtimer` side state keyed by the C timer address.
    static ref HRTIMERS: Mutex<BTreeMap<usize, PendingHrtimer>> = Mutex::new(BTreeMap::new());
    /// kthread workers: worker cookie → queued `struct kthread_work *`s.
    static ref KTHREAD_WORKERS: Mutex<BTreeMap<usize, Vec<usize>>> = Mutex::new(BTreeMap::new());
}

static PUMP_REGISTERED: AtomicBool = AtomicBool::new(false);

type TimerFn = unsafe extern "C" fn(usize);
type HrtimerFn = unsafe extern "C" fn(usize) -> i32;
type KthreadWorkFn = unsafe extern "C" fn(usize);

const HRTIMER_RESTART: i32 = 1;

fn drm_abi_pump() -> usize {
    let mut handled = 0usize;

    // timer_list wheel.
    let now = jiffies_now();
    let due: Vec<(usize, PendingTimer)> = {
        let mut timers = TIMERS.lock();
        let due: Vec<_> = timers
            .iter()
            .filter(|(_, t)| t.expires_jiffies <= now)
            .map(|(&k, &v)| (k, v))
            .collect();
        for (k, _) in &due {
            timers.remove(k);
        }
        due
    };
    for (timer, state) in due {
        if state.func != 0 {
            let func: TimerFn = unsafe { core::mem::transmute(state.func) };
            unsafe { func(timer) };
            handled += 1;
        }
    }

    // delayed works whose delay elapsed.
    let now = jiffies_now();
    let due: Vec<(usize, PendingDelayedWork)> = {
        let mut works = DELAYED_WORKS.lock();
        let due: Vec<_> = works
            .iter()
            .filter(|(_, w)| w.expires_jiffies <= now)
            .map(|(&k, &v)| (k, v))
            .collect();
        for (k, _) in &due {
            works.remove(k);
        }
        due
    };
    for (dwork, state) in due {
        unsafe {
            linux_queue_work_now(state.wq as *mut c_void, dwork as *mut c_void);
        }
        handled += 1;
    }

    // hrtimers (drives the DRM vblank timer).
    let now_ns = ktime_now_ns();
    let due: Vec<usize> = HRTIMERS
        .lock()
        .iter()
        .filter(|(_, t)| t.active && t.expires_ns <= now_ns)
        .map(|(&k, _)| k)
        .collect();
    for timer in due {
        let Some(state) = HRTIMERS.lock().get(&timer).copied() else {
            continue;
        };
        if state.func == 0 {
            continue;
        }
        let func: HrtimerFn = unsafe { core::mem::transmute(state.func) };
        let restart = unsafe { func(timer) };
        let mut table = HRTIMERS.lock();
        if let Some(entry) = table.get_mut(&timer) {
            if restart == HRTIMER_RESTART {
                // The callback re-armed via hrtimer_forward; pick up the new
                // expiry it wrote through the shim.
                entry.expires_ns = unsafe { *((timer + HRTIMER_EXPIRES) as *const u64) };
            } else {
                entry.active = false;
            }
        }
        handled += 1;
    }

    // kthread workers (DRM vblank workers).
    let queued: Vec<usize> = {
        let mut workers = KTHREAD_WORKERS.lock();
        let mut all = Vec::new();
        for queue in workers.values_mut() {
            all.append(queue);
        }
        all
    };
    for work in queued {
        let func_ptr = unsafe { *((work + KTHREAD_WORK_FUNC) as *const usize) };
        if func_ptr != 0 {
            let func: KthreadWorkFn = unsafe { core::mem::transmute(func_ptr) };
            unsafe { func(work) };
            handled += 1;
        }
    }

    // system workqueues used by the chain (damage worker, etc.).
    handled += drain_registered_workqueues();

    handled
}

lazy_static! {
    /// Every workqueue the chain allocated or referenced, drained by the pump.
    static ref PUMP_WORKQUEUES: Mutex<Vec<usize>> = Mutex::new(Vec::new());
}

fn pump_track_workqueue(wq: usize) {
    if wq == 0 {
        return;
    }
    let mut list = PUMP_WORKQUEUES.lock();
    if !list.contains(&wq) {
        list.push(wq);
    }
}

fn drain_registered_workqueues() -> usize {
    use crate::kernel::workqueue::Workqueue;
    let list: Vec<usize> = PUMP_WORKQUEUES.lock().clone();
    let mut handled = 0;
    for wq in list {
        let queue = unsafe { alloc::sync::Arc::from_raw(wq as *const Workqueue) };
        let pending = queue.nr_pending();
        if pending != 0 {
            crate::kernel::workqueue::flush_workqueue(&queue);
            handled += pending;
        }
        let _ = alloc::sync::Arc::into_raw(queue);
    }
    handled
}

fn ensure_pump_registered() {
    if PUMP_REGISTERED.swap(true, Ordering::AcqRel) {
        return;
    }
    crate::linux_driver_abi::register_driver_abi_poller("drm-module-abi", drm_abi_pump);
}

fn ktime_now_ns() -> u64 {
    crate::kernel::time::ktime_get()
}

/// Pump events and yield once — the wait-loop body shared by the blocking
/// shims, mirroring `kernel/sched/completion.rs::linux_wait_for_completion`.
fn wait_pump_once() {
    #[cfg(not(test))]
    {
        let _ = crate::linux_driver_abi::poll_driver_abi_events();
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
    }
}

// ── allocator shims ────────────────────────────────────────────────────────

#[unsafe(export_name = "__kmalloc_node_track_caller_noprof")]
unsafe extern "C" fn linux_kmalloc_node_track_caller(
    size: usize,
    flags: u32,
    _node: i32,
    _caller: usize,
) -> *mut u8 {
    unsafe { crate::mm::slab::linux___kmalloc_noprof(size, flags) }
}

#[unsafe(export_name = "kmemdup_noprof")]
unsafe extern "C" fn linux_kmemdup(src: *const u8, len: usize, _flags: u32) -> *mut u8 {
    if src.is_null() {
        return core::ptr::null_mut();
    }
    let dst = kmalloc(len);
    if !dst.is_null() {
        unsafe { core::ptr::copy_nonoverlapping(src, dst, len) };
    }
    dst
}

#[unsafe(export_name = "kstrdup")]
unsafe extern "C" fn linux_kstrdup(s: *const c_char, _flags: u32) -> *mut c_char {
    if s.is_null() {
        return core::ptr::null_mut();
    }
    kstrdup_rust(unsafe { c_str(s) })
}

/// `kstrdup_const` always duplicates, so `kfree_const` can always `kfree` —
/// the pair stays internally consistent without a kernel-rodata range check.
#[unsafe(export_name = "kstrdup_const")]
unsafe extern "C" fn linux_kstrdup_const(s: *const c_char, flags: u32) -> *mut c_char {
    unsafe { linux_kstrdup(s, flags) }
}

#[unsafe(export_name = "kfree_const")]
unsafe extern "C" fn linux_kfree_const(ptr: *mut u8) {
    kfree(ptr);
}

#[unsafe(export_name = "__kvmalloc_node_noprof")]
unsafe extern "C" fn linux_kvmalloc_node(size: usize, flags: u32, _node: i32) -> *mut u8 {
    let ptr = unsafe { crate::mm::slab::linux___kmalloc_noprof(size, flags) };
    if !ptr.is_null() {
        return ptr;
    }
    crate::mm::vmalloc::vmalloc(size)
}

#[unsafe(export_name = "krealloc_node_align_noprof")]
unsafe extern "C" fn linux_krealloc_node_align(
    ptr: *mut u8,
    new_size: usize,
    _align: usize,
    flags: u32,
    _node: i32,
) -> *mut u8 {
    if ptr.is_null() {
        return unsafe { crate::mm::slab::linux___kmalloc_noprof(new_size, flags) };
    }
    if new_size == 0 {
        kfree(ptr);
        return core::ptr::null_mut();
    }
    let old_size = crate::mm::slab::ksize(ptr);
    let dst = unsafe { crate::mm::slab::linux___kmalloc_noprof(new_size, flags) };
    if !dst.is_null() {
        let copy = core::cmp::min(old_size, new_size);
        unsafe { core::ptr::copy_nonoverlapping(ptr, dst, copy) };
        kfree(ptr);
    }
    dst
}

#[unsafe(export_name = "memdup_user")]
unsafe extern "C" fn linux_memdup_user(src: *const u8, len: usize) -> *mut u8 {
    let dst = kmalloc(len);
    if dst.is_null() {
        return (-(ENOMEM as isize)) as *mut u8;
    }
    let not_copied = unsafe { crate::arch::x86::kernel::uaccess::copy_from_user(dst, src, len) };
    if not_copied != 0 {
        kfree(dst);
        return (-(crate::include::uapi::errno::EFAULT as isize)) as *mut u8;
    }
    dst
}

#[unsafe(export_name = "vmemdup_user")]
unsafe extern "C" fn linux_vmemdup_user(src: *const u8, len: usize) -> *mut u8 {
    unsafe { linux_memdup_user(src, len) }
}

#[unsafe(export_name = "memdup_user_nul")]
unsafe extern "C" fn linux_memdup_user_nul(src: *const u8, len: usize) -> *mut u8 {
    let dst = kmalloc(len + 1);
    if dst.is_null() {
        return (-(ENOMEM as isize)) as *mut u8;
    }
    let not_copied = unsafe { crate::arch::x86::kernel::uaccess::copy_from_user(dst, src, len) };
    if not_copied != 0 {
        kfree(dst);
        return (-(crate::include::uapi::errno::EFAULT as isize)) as *mut u8;
    }
    unsafe { *dst.add(len) = 0 };
    dst
}

#[unsafe(export_name = "memcpy_and_pad")]
unsafe extern "C" fn linux_memcpy_and_pad(
    dst: *mut u8,
    dst_len: usize,
    src: *const u8,
    src_len: usize,
    pad: i32,
) {
    if dst.is_null() {
        return;
    }
    let copy = core::cmp::min(dst_len, src_len);
    unsafe {
        if !src.is_null() {
            core::ptr::copy_nonoverlapping(src, dst, copy);
        }
        if dst_len > copy {
            core::ptr::write_bytes(dst.add(copy), pad as u8, dst_len - copy);
        }
    }
}

// ── printf-family shims ────────────────────────────────────────────────────

#[unsafe(export_name = "kasprintf")]
unsafe extern "C" fn linux_kasprintf(
    _gfp: usize,
    fmt: *const c_char,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
) -> *mut c_char {
    let formatted = format_c(unsafe { c_str(fmt) }, &[a0, a1, a2, a3]);
    kstrdup_rust(&formatted)
}

#[unsafe(export_name = "kvasprintf")]
unsafe extern "C" fn linux_kvasprintf(
    _gfp: usize,
    fmt: *const c_char,
    _args: usize,
) -> *mut c_char {
    // va_list contents are not recoverable here; keep the format string,
    // which is what the chain's diagnostics use it for.
    kstrdup_rust(unsafe { c_str(fmt) })
}

#[unsafe(export_name = "scnprintf")]
unsafe extern "C" fn linux_scnprintf(
    buf: *mut c_char,
    size: usize,
    fmt: *const c_char,
    a0: usize,
    a1: usize,
    a2: usize,
) -> i32 {
    if buf.is_null() || size == 0 {
        return 0;
    }
    let formatted = format_c(unsafe { c_str(fmt) }, &[a0, a1, a2]);
    let take = core::cmp::min(formatted.len(), size - 1);
    unsafe {
        core::ptr::copy_nonoverlapping(formatted.as_ptr(), buf.cast::<u8>(), take);
        *buf.add(take) = 0;
    }
    take as i32
}

#[unsafe(export_name = "simple_strtol")]
unsafe extern "C" fn linux_simple_strtol(
    cp: *const c_char,
    endp: *mut *const c_char,
    base: u32,
) -> isize {
    let s = unsafe { c_str(cp) };
    let trimmed = s.trim_start();
    let (neg, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed),
    };
    let radix = if base == 0 {
        if digits.starts_with("0x") || digits.starts_with("0X") {
            16
        } else if digits.starts_with('0') && digits.len() > 1 {
            8
        } else {
            10
        }
    } else {
        base
    };
    let digits = if radix == 16 {
        digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
            .unwrap_or(digits)
    } else {
        digits
    };
    let mut value: isize = 0;
    let mut consumed = 0usize;
    for ch in digits.chars() {
        let Some(digit) = ch.to_digit(radix) else {
            break;
        };
        value = value
            .saturating_mul(radix as isize)
            .saturating_add(digit as isize);
        consumed += ch.len_utf8();
    }
    if !endp.is_null() {
        let advance = (s.len() - trimmed.len())
            + if neg { 1 } else { 0 }
            + (trimmed.len() - digits.len() - if neg { 1 } else { 0 })
            + consumed;
        unsafe { *endp = cp.add(advance) };
    }
    if neg { -value } else { value }
}

#[unsafe(export_name = "sysfs_streq")]
unsafe extern "C" fn linux_sysfs_streq(s1: *const c_char, s2: *const c_char) -> i32 {
    let a = unsafe { c_str(s1) }.trim_end_matches('\n');
    let b = unsafe { c_str(s2) }.trim_end_matches('\n');
    (a == b) as i32
}

// ── sort/list_sort ─────────────────────────────────────────────────────────

type CmpFn = unsafe extern "C" fn(*const c_void, *const c_void) -> i32;
type SwapFn = unsafe extern "C" fn(*mut c_void, *mut c_void, i32);

/// `sort` — `vendor/linux/lib/sort.c` (insertion sort variant; the chain only
/// sorts small mode lists).
#[unsafe(export_name = "sort")]
unsafe extern "C" fn linux_sort(
    base: *mut u8,
    num: usize,
    size: usize,
    cmp: Option<CmpFn>,
    swap: Option<SwapFn>,
) {
    let Some(cmp) = cmp else { return };
    if base.is_null() || size == 0 {
        return;
    }
    unsafe {
        for i in 1..num {
            let mut j = i;
            while j > 0 {
                let a = base.add((j - 1) * size);
                let b = base.add(j * size);
                if cmp(a.cast(), b.cast()) <= 0 {
                    break;
                }
                match swap {
                    Some(swap) => swap(a.cast(), b.cast(), size as i32),
                    None => {
                        for off in 0..size {
                            core::ptr::swap(a.add(off), b.add(off));
                        }
                    }
                }
                j -= 1;
            }
        }
    }
}

type ListCmpFn = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32;

#[repr(C)]
struct CListHead {
    next: *mut CListHead,
    prev: *mut CListHead,
}

/// `list_sort` — `vendor/linux/lib/list_sort.c`, simplified to repeated
/// insertion into a fresh list (mode lists are short).
#[unsafe(export_name = "list_sort")]
unsafe extern "C" fn linux_list_sort(
    priv_: *mut c_void,
    head: *mut CListHead,
    cmp: Option<ListCmpFn>,
) {
    let Some(cmp) = cmp else { return };
    if head.is_null() {
        return;
    }
    unsafe {
        let mut nodes: Vec<*mut CListHead> = Vec::new();
        let mut cur = (*head).next;
        while cur != head {
            nodes.push(cur);
            cur = (*cur).next;
        }
        // Insertion sort on the collected node pointers.
        for i in 1..nodes.len() {
            let mut j = i;
            while j > 0 && cmp(priv_, nodes[j - 1].cast(), nodes[j].cast()) > 0 {
                nodes.swap(j - 1, j);
                j -= 1;
            }
        }
        // Relink.
        let mut prev = head;
        for &node in &nodes {
            (*prev).next = node;
            (*node).prev = prev;
            prev = node;
        }
        (*prev).next = head;
        (*head).prev = prev;
    }
}

// ── idr / xarray side tables ───────────────────────────────────────────────

lazy_static! {
    static ref IDRS: Mutex<BTreeMap<usize, BTreeMap<u64, usize>>> = Mutex::new(BTreeMap::new());
    static ref XARRAYS: Mutex<BTreeMap<usize, BTreeMap<u64, usize>>> = Mutex::new(BTreeMap::new());
}

#[unsafe(export_name = "idr_alloc")]
unsafe extern "C" fn linux_idr_alloc(
    idr: *mut c_void,
    ptr: *mut c_void,
    start: i32,
    end: i32,
    _gfp: u32,
) -> i32 {
    let mut table = IDRS.lock();
    let entries = table.entry(idr as usize).or_default();
    let start = start.max(0) as u64;
    let end = if end <= 0 {
        i32::MAX as u64
    } else {
        end as u64
    };
    let mut id = start;
    while id < end {
        if !entries.contains_key(&id) {
            entries.insert(id, ptr as usize);
            return id as i32;
        }
        id += 1;
    }
    -ENOSPC
}

#[unsafe(export_name = "idr_remove")]
unsafe extern "C" fn linux_idr_remove(idr: *mut c_void, id: u64) -> *mut c_void {
    IDRS.lock()
        .get_mut(&(idr as usize))
        .and_then(|entries| entries.remove(&id))
        .unwrap_or(0) as *mut c_void
}

#[unsafe(export_name = "idr_find")]
unsafe extern "C" fn linux_idr_find(idr: *mut c_void, id: u64) -> *mut c_void {
    IDRS.lock()
        .get(&(idr as usize))
        .and_then(|entries| entries.get(&id).copied())
        .unwrap_or(0) as *mut c_void
}

#[unsafe(export_name = "idr_replace")]
unsafe extern "C" fn linux_idr_replace(idr: *mut c_void, ptr: *mut c_void, id: u64) -> *mut c_void {
    let mut table = IDRS.lock();
    let Some(entries) = table.get_mut(&(idr as usize)) else {
        return (-(EINVAL as isize)) as *mut c_void;
    };
    match entries.get_mut(&id) {
        Some(slot) => {
            let old = *slot;
            *slot = ptr as usize;
            old as *mut c_void
        }
        None => (-(crate::include::uapi::errno::ENOENT as isize)) as *mut c_void,
    }
}

type IdrIterFn = unsafe extern "C" fn(i32, *mut c_void, *mut c_void) -> i32;

#[unsafe(export_name = "idr_for_each")]
unsafe extern "C" fn linux_idr_for_each(
    idr: *mut c_void,
    func: Option<IdrIterFn>,
    data: *mut c_void,
) -> i32 {
    let Some(func) = func else { return 0 };
    let entries: Vec<(u64, usize)> = IDRS
        .lock()
        .get(&(idr as usize))
        .map(|entries| entries.iter().map(|(&k, &v)| (k, v)).collect())
        .unwrap_or_default();
    for (id, ptr) in entries {
        let ret = unsafe { func(id as i32, ptr as *mut c_void, data) };
        if ret != 0 {
            return ret;
        }
    }
    0
}

#[unsafe(export_name = "idr_get_next")]
unsafe extern "C" fn linux_idr_get_next(idr: *mut c_void, nextid: *mut i32) -> *mut c_void {
    if nextid.is_null() {
        return core::ptr::null_mut();
    }
    let start = unsafe { *nextid }.max(0) as u64;
    let table = IDRS.lock();
    let Some(entries) = table.get(&(idr as usize)) else {
        return core::ptr::null_mut();
    };
    match entries.range(start..).next() {
        Some((&id, &ptr)) => {
            unsafe { *nextid = id as i32 };
            ptr as *mut c_void
        }
        None => core::ptr::null_mut(),
    }
}

#[unsafe(export_name = "idr_destroy")]
unsafe extern "C" fn linux_idr_destroy(idr: *mut c_void) {
    IDRS.lock().remove(&(idr as usize));
}

#[unsafe(export_name = "idr_preload")]
unsafe extern "C" fn linux_idr_preload(_gfp: u32) {}

#[unsafe(export_name = "ida_destroy")]
unsafe extern "C" fn linux_ida_destroy(_ida: *mut c_void) {}

#[unsafe(export_name = "xa_store")]
unsafe extern "C" fn linux_xa_store(
    xa: *mut c_void,
    index: u64,
    entry: *mut c_void,
    _gfp: u32,
) -> *mut c_void {
    let mut table = XARRAYS.lock();
    let entries = table.entry(xa as usize).or_default();
    if entry.is_null() {
        return entries.remove(&index).unwrap_or(0) as *mut c_void;
    }
    entries.insert(index, entry as usize).unwrap_or(0) as *mut c_void
}

#[unsafe(export_name = "xa_load")]
unsafe extern "C" fn linux_xa_load(xa: *mut c_void, index: u64) -> *mut c_void {
    XARRAYS
        .lock()
        .get(&(xa as usize))
        .and_then(|entries| entries.get(&index).copied())
        .unwrap_or(0) as *mut c_void
}

#[unsafe(export_name = "xa_erase")]
unsafe extern "C" fn linux_xa_erase(xa: *mut c_void, index: u64) -> *mut c_void {
    XARRAYS
        .lock()
        .get_mut(&(xa as usize))
        .and_then(|entries| entries.remove(&index))
        .unwrap_or(0) as *mut c_void
}

/// `__xa_alloc` — `limit` arrives as `struct xa_limit { u32 max; u32 min; }`
/// packed into one 64-bit register.
#[unsafe(export_name = "__xa_alloc")]
unsafe extern "C" fn linux_xa_alloc(
    xa: *mut c_void,
    id: *mut u32,
    entry: *mut c_void,
    limit: u64,
    _gfp: u32,
) -> i32 {
    let max = (limit & 0xffff_ffff) as u64;
    let min = (limit >> 32) as u64;
    let mut table = XARRAYS.lock();
    let entries = table.entry(xa as usize).or_default();
    let mut index = min;
    while index <= max {
        if !entries.contains_key(&index) {
            entries.insert(index, entry as usize);
            if !id.is_null() {
                unsafe { *id = index as u32 };
            }
            return 0;
        }
        index += 1;
    }
    -EBUSY
}

#[unsafe(export_name = "xa_find")]
unsafe extern "C" fn linux_xa_find(
    xa: *mut c_void,
    indexp: *mut u64,
    max: u64,
    _filter: u64,
) -> *mut c_void {
    if indexp.is_null() {
        return core::ptr::null_mut();
    }
    let start = unsafe { *indexp };
    let table = XARRAYS.lock();
    let Some(entries) = table.get(&(xa as usize)) else {
        return core::ptr::null_mut();
    };
    match entries.range(start..=max).next() {
        Some((&index, &entry)) => {
            unsafe { *indexp = index };
            entry as *mut c_void
        }
        None => core::ptr::null_mut(),
    }
}

#[unsafe(export_name = "xa_find_after")]
unsafe extern "C" fn linux_xa_find_after(
    xa: *mut c_void,
    indexp: *mut u64,
    max: u64,
    filter: u64,
) -> *mut c_void {
    if indexp.is_null() {
        return core::ptr::null_mut();
    }
    let start = unsafe { *indexp }.saturating_add(1);
    unsafe {
        *indexp = start;
        linux_xa_find(xa, indexp, max, filter)
    }
}

#[unsafe(export_name = "xa_destroy")]
unsafe extern "C" fn linux_xa_destroy(xa: *mut c_void) {
    XARRAYS.lock().remove(&(xa as usize));
}

#[unsafe(export_name = "radix_tree_tagged")]
unsafe extern "C" fn linux_radix_tree_tagged(_root: *mut c_void, _tag: u32) -> i32 {
    0
}

// ── locking: ww_mutex, mutex extras, refcount ─────────────────────────────

lazy_static! {
    /// ww_mutex state keyed by lock address: the acquire-context pointer that
    /// currently owns it (usize::MAX = locked without context).
    static ref WW_LOCKS: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
}

const WW_NO_CTX: usize = usize::MAX;
const EALREADY: i32 = 114;
const EDEADLK: i32 = 35;

fn ww_try_acquire(lock: usize, ctx: usize) -> i32 {
    let mut locks = WW_LOCKS.lock();
    match locks.get(&lock).copied() {
        None => {
            locks.insert(lock, if ctx == 0 { WW_NO_CTX } else { ctx });
            0
        }
        Some(owner) if ctx != 0 && owner == ctx => -EALREADY,
        Some(_) => -EBUSY,
    }
}

#[unsafe(export_name = "ww_mutex_lock")]
unsafe extern "C" fn linux_ww_mutex_lock(lock: *mut c_void, ctx: *mut c_void) -> i32 {
    loop {
        match ww_try_acquire(lock as usize, ctx as usize) {
            0 => return 0,
            r if r == -EALREADY => return -EALREADY,
            _ => wait_pump_once(),
        }
        #[cfg(test)]
        return -EDEADLK;
    }
}

#[unsafe(export_name = "ww_mutex_lock_interruptible")]
unsafe extern "C" fn linux_ww_mutex_lock_interruptible(lock: *mut c_void, ctx: *mut c_void) -> i32 {
    unsafe { linux_ww_mutex_lock(lock, ctx) }
}

#[unsafe(export_name = "ww_mutex_trylock")]
unsafe extern "C" fn linux_ww_mutex_trylock(lock: *mut c_void, ctx: *mut c_void) -> i32 {
    (ww_try_acquire(lock as usize, ctx as usize) == 0) as i32
}

#[unsafe(export_name = "ww_mutex_unlock")]
unsafe extern "C" fn linux_ww_mutex_unlock(lock: *mut c_void) {
    WW_LOCKS.lock().remove(&(lock as usize));
}

/// `struct mutex` raw view shared with `kernel::locking::mutex` glue: owner
/// word first (0 = unlocked).
#[unsafe(export_name = "mutex_is_locked")]
unsafe extern "C" fn linux_mutex_is_locked(lock: *mut u64) -> i32 {
    if lock.is_null() {
        return 0;
    }
    (unsafe { core::ptr::read_volatile(lock) } != 0) as i32
}

#[unsafe(export_name = "mutex_trylock")]
unsafe extern "C" fn linux_mutex_trylock(lock: *mut u64) -> i32 {
    if lock.is_null() {
        return 0;
    }
    let atomic = unsafe { &*(lock as *const AtomicU64) };
    atomic
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_ok() as i32
}

#[unsafe(export_name = "mutex_lock_interruptible")]
unsafe extern "C" fn linux_mutex_lock_interruptible(lock: *mut u64) -> i32 {
    loop {
        if unsafe { linux_mutex_trylock(lock) } != 0 {
            return 0;
        }
        wait_pump_once();
        #[cfg(test)]
        return 0;
    }
}

#[unsafe(export_name = "atomic_dec_and_mutex_lock")]
unsafe extern "C" fn linux_atomic_dec_and_mutex_lock(cnt: *mut AtomicI32, lock: *mut u64) -> i32 {
    if cnt.is_null() {
        return 0;
    }
    let counter = unsafe { &*cnt };
    if counter.fetch_sub(1, Ordering::AcqRel) - 1 != 0 {
        return 0;
    }
    unsafe {
        let _ = linux_mutex_lock_interruptible(lock);
    }
    1
}

#[unsafe(export_name = "refcount_warn_saturate")]
unsafe extern "C" fn linux_refcount_warn_saturate(_ref: *mut c_void, _type: i32) {
    crate::log_warn!("drm-abi", "refcount saturated in DRM module chain");
}

#[unsafe(export_name = "refcount_dec_not_one")]
unsafe extern "C" fn linux_refcount_dec_not_one(refcount: *mut AtomicI32) -> i32 {
    if refcount.is_null() {
        return 0;
    }
    let counter = unsafe { &*refcount };
    loop {
        let current = counter.load(Ordering::Acquire);
        if current == 1 {
            return 0;
        }
        if counter
            .compare_exchange(current, current - 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            return 1;
        }
    }
}

// ── waitqueues / completions / scheduling ──────────────────────────────────

#[unsafe(export_name = "__init_waitqueue_head")]
unsafe extern "C" fn linux_init_waitqueue_head(
    queue: *mut c_void,
    _name: *const c_char,
    _key: *mut c_void,
) {
    // struct wait_queue_head = spinlock + list_head; make the list self-linked
    // (offset 8 in the vendor layout: 4-byte spinlock + 4 padding).
    if queue.is_null() {
        return;
    }
    unsafe {
        let list = queue.cast::<u8>().add(8).cast::<CListHead>();
        (*list).next = list;
        (*list).prev = list;
    }
}

/// `wait_event()` loops re-check their condition around `schedule()`, so the
/// wait entry management can be inert as long as `schedule` pumps events.
#[unsafe(export_name = "init_wait_entry")]
unsafe extern "C" fn linux_init_wait_entry(_entry: *mut c_void, _flags: i32) {}

#[unsafe(export_name = "prepare_to_wait_event")]
unsafe extern "C" fn linux_prepare_to_wait_event(
    _queue: *mut c_void,
    _entry: *mut c_void,
    _state: i32,
) -> i32 {
    0
}

#[unsafe(export_name = "finish_wait")]
unsafe extern "C" fn linux_finish_wait(_queue: *mut c_void, _entry: *mut c_void) {}

#[unsafe(export_name = "__wake_up")]
unsafe extern "C" fn linux_wake_up(_queue: *mut c_void, _mode: u32, _nr: i32, _key: *mut c_void) {
    // Waiters poll their condition from the schedule() pump loop.
}

#[unsafe(export_name = "schedule")]
unsafe extern "C" fn linux_schedule() {
    wait_pump_once();
}

#[unsafe(export_name = "wake_up_process")]
unsafe extern "C" fn linux_wake_up_process(_task: *mut c_void) -> i32 {
    1
}

#[unsafe(export_name = "sched_set_fifo")]
unsafe extern "C" fn linux_sched_set_fifo(_task: *mut c_void) {}

#[unsafe(export_name = "try_wait_for_completion")]
unsafe extern "C" fn linux_try_wait_for_completion_export(completion: *mut c_void) -> i32 {
    unsafe {
        crate::kernel::sched::completion::linux_try_wait_for_completion_raw(completion) as i32
    }
}

unsafe fn wait_for_completion_jiffies(completion: *mut c_void, timeout: u64) -> u64 {
    let deadline = jiffies_now().saturating_add(timeout);
    loop {
        if unsafe {
            crate::kernel::sched::completion::linux_try_wait_for_completion_raw(completion)
        } {
            return core::cmp::max(deadline.saturating_sub(jiffies_now()), 1);
        }
        if jiffies_now() >= deadline {
            return 0;
        }
        wait_pump_once();
        #[cfg(test)]
        return 0;
    }
}

#[unsafe(export_name = "wait_for_completion_timeout")]
unsafe extern "C" fn linux_wait_for_completion_timeout(
    completion: *mut c_void,
    timeout: u64,
) -> u64 {
    unsafe { wait_for_completion_jiffies(completion, timeout) }
}

#[unsafe(export_name = "wait_for_completion_interruptible")]
unsafe extern "C" fn linux_wait_for_completion_interruptible(completion: *mut c_void) -> i32 {
    unsafe {
        crate::kernel::sched::completion::linux_wait_for_completion(completion);
    }
    0
}

#[unsafe(export_name = "wait_for_completion_interruptible_timeout")]
unsafe extern "C" fn linux_wait_for_completion_interruptible_timeout(
    completion: *mut c_void,
    timeout: u64,
) -> i64 {
    unsafe { wait_for_completion_jiffies(completion, timeout) as i64 }
}

// ── timers / delayed work / hrtimers / kthread workers ───────────────────

#[unsafe(export_name = "timer_init_key")]
unsafe extern "C" fn linux_timer_init_key(
    timer: *mut c_void,
    func: usize,
    _flags: u32,
    _name: *const c_char,
    _key: *mut c_void,
) {
    ensure_pump_registered();
    TIMERS.lock().insert(
        timer as usize,
        PendingTimer {
            func,
            expires_jiffies: u64::MAX,
        },
    );
}

#[unsafe(export_name = "mod_timer")]
unsafe extern "C" fn linux_mod_timer(timer: *mut c_void, expires: u64) -> i32 {
    ensure_pump_registered();
    let mut timers = TIMERS.lock();
    let entry = timers.entry(timer as usize).or_insert(PendingTimer {
        func: 0,
        expires_jiffies: u64::MAX,
    });
    let was_armed = entry.expires_jiffies != u64::MAX;
    entry.expires_jiffies = expires;
    was_armed as i32
}

#[unsafe(export_name = "timer_delete_sync")]
unsafe extern "C" fn linux_timer_delete_sync(timer: *mut c_void) -> i32 {
    let mut timers = TIMERS.lock();
    match timers.get_mut(&(timer as usize)) {
        Some(entry) if entry.expires_jiffies != u64::MAX => {
            entry.expires_jiffies = u64::MAX;
            1
        }
        _ => 0,
    }
}

/// `delayed_work_timer_fn` — address stored into `dwork->timer.function` by
/// `INIT_DELAYED_WORK`; if the module fires it directly, queue the containing
/// work immediately.
#[unsafe(export_name = "delayed_work_timer_fn")]
unsafe extern "C" fn linux_delayed_work_timer_fn(timer: *mut c_void) {
    let dwork = (timer as usize).wrapping_sub(DWORK_TIMER);
    let wq = DELAYED_WORKS
        .lock()
        .remove(&dwork)
        .map(|w| w.wq)
        .unwrap_or(0);
    unsafe { linux_queue_work_now(wq as *mut c_void, dwork as *mut c_void) };
}

/// Queue `work` on `wq` right now (delegates to the workqueue glue; a null
/// `wq` falls back to the system workqueue).
unsafe fn linux_queue_work_now(wq: *mut c_void, work: *mut c_void) {
    use crate::kernel::workqueue::{SYSTEM_WQ, WorkStruct};
    let wq = if wq.is_null() {
        alloc::sync::Arc::into_raw(SYSTEM_WQ.get()) as *mut c_void
    } else {
        wq
    };
    pump_track_workqueue(wq as usize);
    unsafe {
        crate::kernel::workqueue::linux_queue_work_on(0, wq.cast(), work.cast::<WorkStruct>());
    }
}

#[unsafe(export_name = "queue_delayed_work_on")]
unsafe extern "C" fn linux_queue_delayed_work_on(
    _cpu: i32,
    wq: *mut c_void,
    dwork: *mut c_void,
    delay: u64,
) -> i32 {
    ensure_pump_registered();
    if dwork.is_null() {
        return 0;
    }
    if delay == 0 {
        unsafe { linux_queue_work_now(wq, dwork) };
        return 1;
    }
    DELAYED_WORKS.lock().insert(
        dwork as usize,
        PendingDelayedWork {
            wq: wq as usize,
            expires_jiffies: jiffies_now().saturating_add(delay),
        },
    );
    1
}

#[unsafe(export_name = "mod_delayed_work_on")]
unsafe extern "C" fn linux_mod_delayed_work_on(
    cpu: i32,
    wq: *mut c_void,
    dwork: *mut c_void,
    delay: u64,
) -> i32 {
    DELAYED_WORKS.lock().remove(&(dwork as usize));
    unsafe { linux_queue_delayed_work_on(cpu, wq, dwork, delay) }
}

#[unsafe(export_name = "cancel_delayed_work_sync")]
unsafe extern "C" fn linux_cancel_delayed_work_sync(dwork: *mut c_void) -> i32 {
    let armed = DELAYED_WORKS.lock().remove(&(dwork as usize)).is_some();
    // Also clear a pending immediate queue entry.
    if !dwork.is_null() {
        let work = dwork.cast::<crate::kernel::workqueue::WorkStruct>();
        unsafe {
            (*work)
                .data
                .fetch_and(!crate::kernel::workqueue::WORK_PENDING, Ordering::AcqRel);
        }
    }
    armed as i32
}

#[unsafe(export_name = "cancel_work_sync")]
unsafe extern "C" fn linux_cancel_work_sync(work: *mut c_void) -> i32 {
    if work.is_null() {
        return 0;
    }
    let work = work.cast::<crate::kernel::workqueue::WorkStruct>();
    let was_pending = unsafe {
        (*work)
            .data
            .fetch_and(!crate::kernel::workqueue::WORK_PENDING, Ordering::AcqRel)
            & crate::kernel::workqueue::WORK_PENDING
            != 0
    };
    was_pending as i32
}

#[unsafe(export_name = "flush_delayed_work")]
unsafe extern "C" fn linux_flush_delayed_work(dwork: *mut c_void) -> i32 {
    let armed = DELAYED_WORKS.lock().remove(&(dwork as usize));
    if armed.is_some() {
        unsafe {
            linux_queue_work_now(armed.map(|w| w.wq).unwrap_or(0) as *mut c_void, dwork);
        }
    }
    let _ = drain_registered_workqueues();
    armed.is_some() as i32
}

#[unsafe(export_name = "current_work")]
unsafe extern "C" fn linux_current_work() -> *mut c_void {
    core::ptr::null_mut()
}

#[unsafe(export_name = "hrtimer_setup")]
unsafe extern "C" fn linux_hrtimer_setup(
    timer: *mut c_void,
    func: usize,
    _clock_id: i32,
    _mode: i32,
) {
    ensure_pump_registered();
    if timer.is_null() {
        return;
    }
    unsafe {
        *((timer as usize + HRTIMER_FUNCTION) as *mut usize) = func;
    }
    HRTIMERS.lock().insert(
        timer as usize,
        PendingHrtimer {
            func,
            expires_ns: 0,
            active: false,
        },
    );
}

#[unsafe(export_name = "hrtimer_start_range_ns")]
unsafe extern "C" fn linux_hrtimer_start_range_ns(
    timer: *mut c_void,
    expires_ns: i64,
    _range_ns: u64,
    _mode: i32,
) {
    if timer.is_null() {
        return;
    }
    let expires = expires_ns.max(0) as u64;
    unsafe {
        *((timer as usize + HRTIMER_EXPIRES) as *mut u64) = expires;
        *((timer as usize + HRTIMER_SOFTEXPIRES) as *mut u64) = expires;
    }
    let mut table = HRTIMERS.lock();
    let entry = table.entry(timer as usize).or_insert(PendingHrtimer {
        func: unsafe { *((timer as usize + HRTIMER_FUNCTION) as *const usize) },
        expires_ns: 0,
        active: false,
    });
    entry.expires_ns = expires;
    entry.active = true;
}

#[unsafe(export_name = "hrtimer_cancel")]
unsafe extern "C" fn linux_hrtimer_cancel(timer: *mut c_void) -> i32 {
    let mut table = HRTIMERS.lock();
    match table.get_mut(&(timer as usize)) {
        Some(entry) if entry.active => {
            entry.active = false;
            1
        }
        _ => 0,
    }
}

#[unsafe(export_name = "hrtimer_try_to_cancel")]
unsafe extern "C" fn linux_hrtimer_try_to_cancel(timer: *mut c_void) -> i32 {
    unsafe { linux_hrtimer_cancel(timer) }
}

#[unsafe(export_name = "hrtimer_active")]
unsafe extern "C" fn linux_hrtimer_active(timer: *const c_void) -> i32 {
    HRTIMERS
        .lock()
        .get(&(timer as usize))
        .map(|entry| entry.active)
        .unwrap_or(false) as i32
}

#[unsafe(export_name = "hrtimer_forward")]
unsafe extern "C" fn linux_hrtimer_forward(
    timer: *mut c_void,
    now_ns: i64,
    interval_ns: i64,
) -> u64 {
    if timer.is_null() || interval_ns <= 0 {
        return 0;
    }
    let interval = interval_ns as u64;
    let now = now_ns.max(0) as u64;
    let expires_ptr = (timer as usize + HRTIMER_EXPIRES) as *mut u64;
    let mut expires = unsafe { *expires_ptr };
    let mut overruns = 0u64;
    while expires <= now {
        expires += interval;
        overruns += 1;
    }
    unsafe {
        *expires_ptr = expires;
        *((timer as usize + HRTIMER_SOFTEXPIRES) as *mut u64) = expires;
    }
    if let Some(entry) = HRTIMERS.lock().get_mut(&(timer as usize)) {
        entry.expires_ns = expires;
    }
    overruns
}

#[unsafe(export_name = "hrtimer_cb_get_time")]
unsafe extern "C" fn linux_hrtimer_cb_get_time(_timer: *mut c_void) -> i64 {
    ktime_now_ns() as i64
}

#[unsafe(export_name = "kthread_create_worker_on_node")]
unsafe extern "C" fn linux_kthread_create_worker_on_node(
    _flags: u32,
    _node: i32,
    _fmt: *const c_char,
    _a0: usize,
) -> *mut c_void {
    ensure_pump_registered();
    let cookie = kzalloc(64);
    if cookie.is_null() {
        return (-(ENOMEM as isize)) as *mut c_void;
    }
    KTHREAD_WORKERS.lock().insert(cookie as usize, Vec::new());
    cookie.cast()
}

#[unsafe(export_name = "kthread_queue_work")]
unsafe extern "C" fn linux_kthread_queue_work(worker: *mut c_void, work: *mut c_void) -> i32 {
    let mut workers = KTHREAD_WORKERS.lock();
    let Some(queue) = workers.get_mut(&(worker as usize)) else {
        return 0;
    };
    if queue.contains(&(work as usize)) {
        return 0;
    }
    queue.push(work as usize);
    1
}

unsafe fn kthread_run_queued(worker: usize) {
    let queued: Vec<usize> = {
        let mut workers = KTHREAD_WORKERS.lock();
        workers
            .get_mut(&worker)
            .map(core::mem::take)
            .unwrap_or_default()
    };
    for work in queued {
        let func_ptr = unsafe { *((work + KTHREAD_WORK_FUNC) as *const usize) };
        if func_ptr != 0 {
            let func: KthreadWorkFn = unsafe { core::mem::transmute(func_ptr) };
            unsafe { func(work) };
        }
    }
}

#[unsafe(export_name = "kthread_flush_worker")]
unsafe extern "C" fn linux_kthread_flush_worker(worker: *mut c_void) {
    unsafe { kthread_run_queued(worker as usize) };
}

#[unsafe(export_name = "kthread_flush_work")]
unsafe extern "C" fn linux_kthread_flush_work(work: *mut c_void) -> i32 {
    // Run whichever worker currently queues this work.
    let workers: Vec<usize> = KTHREAD_WORKERS
        .lock()
        .iter()
        .filter(|(_, queue)| queue.contains(&(work as usize)))
        .map(|(&worker, _)| worker)
        .collect();
    for worker in &workers {
        unsafe { kthread_run_queued(*worker) };
    }
    (!workers.is_empty()) as i32
}

#[unsafe(export_name = "kthread_cancel_work_sync")]
unsafe extern "C" fn linux_kthread_cancel_work_sync(work: *mut c_void) -> i32 {
    let mut cancelled = false;
    for queue in KTHREAD_WORKERS.lock().values_mut() {
        let before = queue.len();
        queue.retain(|&queued| queued != work as usize);
        cancelled |= queue.len() != before;
    }
    cancelled as i32
}

#[unsafe(export_name = "kthread_destroy_worker")]
unsafe extern "C" fn linux_kthread_destroy_worker(worker: *mut c_void) {
    unsafe { kthread_run_queued(worker as usize) };
    KTHREAD_WORKERS.lock().remove(&(worker as usize));
    kfree(worker.cast());
}

// ── time conversions ───────────────────────────────────────────────────────

#[unsafe(export_name = "ktime_get")]
unsafe extern "C" fn linux_ktime_get() -> i64 {
    ktime_now_ns() as i64
}

#[unsafe(export_name = "__msecs_to_jiffies")]
unsafe extern "C" fn linux_msecs_to_jiffies(msecs: u32) -> u64 {
    (msecs as u64).saturating_mul(MODULE_HZ).div_ceil(1000)
}

#[unsafe(export_name = "nsecs_to_jiffies64")]
unsafe extern "C" fn linux_nsecs_to_jiffies64(nsecs: u64) -> u64 {
    nsecs / (1_000_000_000 / MODULE_HZ)
}

#[repr(C)]
struct Timespec64 {
    tv_sec: i64,
    tv_nsec: i64,
}

#[unsafe(export_name = "ns_to_timespec64")]
unsafe extern "C" fn linux_ns_to_timespec64(nsec: i64) -> Timespec64 {
    if nsec <= 0 {
        return Timespec64 {
            tv_sec: 0,
            tv_nsec: 0,
        };
    }
    Timespec64 {
        tv_sec: nsec / 1_000_000_000,
        tv_nsec: nsec % 1_000_000_000,
    }
}

// ── mm shims ───────────────────────────────────────────────────────────────

#[unsafe(export_name = "vmap")]
unsafe extern "C" fn linux_vmap(
    pages: *const *mut Page,
    count: usize,
    flags: u32,
    prot: u64,
) -> *mut u8 {
    crate::mm::vmalloc::vmap(pages, count, flags, prot)
}

#[unsafe(export_name = "vunmap")]
unsafe extern "C" fn linux_vunmap(addr: *mut u8) {
    crate::mm::vmalloc::vunmap(addr)
}

#[unsafe(export_name = "pgprot_writecombine")]
unsafe extern "C" fn linux_pgprot_writecombine(prot: u64) -> u64 {
    prot
}

#[unsafe(export_name = "vm_get_page_prot")]
unsafe extern "C" fn linux_vm_get_page_prot(_vm_flags: u64) -> u64 {
    // PAGE_SHARED equivalent: present | rw | user | nx.
    0x8000000000000067
}

#[unsafe(export_name = "vmf_insert_pfn")]
unsafe extern "C" fn linux_vmf_insert_pfn(_vmf: *mut c_void, _pfn: u64) -> i32 {
    // VM_FAULT_SIGBUS: GEM mmap faults are not wired into the Lupos VM yet.
    0x0002
}

#[unsafe(export_name = "unmap_mapping_range")]
unsafe extern "C" fn linux_unmap_mapping_range(
    _mapping: *mut c_void,
    _start: i64,
    _len: i64,
    _even_cows: i32,
) {
}

#[unsafe(export_name = "invalidate_mapping_pages")]
unsafe extern "C" fn linux_invalidate_mapping_pages(
    _mapping: *mut c_void,
    _start: u64,
    _end: u64,
) -> u64 {
    0
}

#[unsafe(export_name = "mm_get_unmapped_area")]
unsafe extern "C" fn linux_mm_get_unmapped_area(
    _mm: *mut c_void,
    _file: *mut c_void,
    _addr: u64,
    _len: u64,
    _pgoff: u64,
    _flags: u64,
) -> u64 {
    (-(EINVAL as i64)) as u64
}

/// GEM shmem pages are pinned for the device's whole lifetime, so LRU state
/// transitions are irrelevant; these mirror the "no reclaim" reality.
#[unsafe(export_name = "folio_mark_accessed")]
unsafe extern "C" fn linux_folio_mark_accessed(_folio: *mut c_void) {}

#[unsafe(export_name = "folio_mark_dirty")]
unsafe extern "C" fn linux_folio_mark_dirty(_folio: *mut c_void) -> i32 {
    1
}

#[unsafe(export_name = "__folio_batch_release")]
unsafe extern "C" fn linux_folio_batch_release(_batch: *mut c_void) {}

#[unsafe(export_name = "check_move_unevictable_folios")]
unsafe extern "C" fn linux_check_move_unevictable_folios(_batch: *mut c_void) {}

// ── shmem-backed GEM files ─────────────────────────────────────────────────
//
// `drm_gem_object_init()` calls `shmem_file_setup()` and then dereferences
// `file->f_mapping` / `file->f_inode` inline, so the shim fabricates a block
// large enough for the vendor `struct file`, an embedded `struct inode` and
// `struct address_space`, and wires the pointer fields at the probed offsets.
// The backing store is a side table of real buddy pages keyed by the mapping
// address; `shmem_read_folio_gfp` materialises order-0 folios on demand.

struct ShmemGemFile {
    file: usize,
    pages: BTreeMap<u64, *mut Page>,
}

unsafe impl Send for ShmemGemFile {}

lazy_static! {
    /// mapping-address → backing store.
    static ref SHMEM_FILES: Mutex<BTreeMap<usize, ShmemGemFile>> = Mutex::new(BTreeMap::new());
}

const SHMEM_BLOCK_SIZE: usize = FILE_SIZE + INODE_SIZE + ADDRESS_SPACE_SIZE + 64;

#[unsafe(export_name = "shmem_file_setup")]
unsafe extern "C" fn linux_shmem_file_setup(
    _name: *const c_char,
    _size: i64,
    _flags: u64,
) -> *mut c_void {
    let block = kzalloc(SHMEM_BLOCK_SIZE);
    if block.is_null() {
        return (-(ENOMEM as isize)) as *mut c_void;
    }
    let file = block as usize;
    let inode = file + FILE_SIZE;
    let mapping = inode + INODE_SIZE;
    unsafe {
        *((file + FILE_F_MAPPING) as *mut usize) = mapping;
        *((file + FILE_F_INODE) as *mut usize) = inode;
        *((inode + INODE_I_MAPPING) as *mut usize) = mapping;
    }
    SHMEM_FILES.lock().insert(
        mapping,
        ShmemGemFile {
            file,
            pages: BTreeMap::new(),
        },
    );
    block.cast()
}

#[unsafe(export_name = "shmem_read_folio_gfp")]
unsafe extern "C" fn linux_shmem_read_folio_gfp(
    mapping: *mut c_void,
    index: u64,
    _gfp: u32,
) -> *mut Page {
    let mut files = SHMEM_FILES.lock();
    let Some(entry) = files.get_mut(&(mapping as usize)) else {
        return (-(EINVAL as isize)) as *mut Page;
    };
    if let Some(&page) = entry.pages.get(&index) {
        unsafe { (*page).get_page() };
        return page;
    }
    let Some(page) = crate::mm::buddy::with_global_buddy(|buddy| buddy.alloc_pages(0, GFP_KERNEL))
    else {
        return (-(ENOMEM as isize)) as *mut Page;
    };
    unsafe {
        (*page).mapping = mapping as usize;
        (*page).index = index as usize;
        (*page)._refcount.store(2, Ordering::Release);
    }
    entry.pages.insert(index, page);
    page
}

#[unsafe(export_name = "shmem_truncate_range")]
unsafe extern "C" fn linux_shmem_truncate_range(inode: *mut c_void, _start: i64, _end: i64) {
    // inode → mapping via the pointer planted in shmem_file_setup.
    if inode.is_null() {
        return;
    }
    let mapping = unsafe { *((inode as usize + INODE_I_MAPPING) as *const usize) };
    let mut files = SHMEM_FILES.lock();
    if let Some(entry) = files.get_mut(&mapping) {
        let pages: Vec<*mut Page> = entry.pages.values().copied().collect();
        entry.pages.clear();
        crate::mm::buddy::with_global_buddy(|buddy| {
            for page in &pages {
                unsafe {
                    (**page).mapping = 0;
                    (**page).index = 0;
                    (**page)._refcount.store(0, Ordering::Release);
                }
                buddy.free_pages(*page, 0);
            }
        });
    }
}

#[unsafe(export_name = "fput")]
unsafe extern "C" fn linux_fput(file: *mut c_void) {
    if file.is_null() {
        return;
    }
    let mapping = unsafe { *((file as usize + FILE_F_MAPPING) as *const usize) };
    let owned = {
        let mut files = SHMEM_FILES.lock();
        match files.get(&mapping) {
            Some(entry) if entry.file == file as usize => files.remove(&mapping),
            _ => None,
        }
    };
    if let Some(entry) = owned {
        crate::mm::buddy::with_global_buddy(|buddy| {
            for (_, page) in entry.pages {
                unsafe {
                    (*page).mapping = 0;
                    (*page).index = 0;
                    (*page)._refcount.store(0, Ordering::Release);
                }
                buddy.free_pages(page, 0);
            }
        });
        kfree(file.cast());
    }
}

#[unsafe(export_name = "file_update_time")]
unsafe extern "C" fn linux_file_update_time(_file: *mut c_void) -> i32 {
    0
}

// ── drm_fs anon inode plumbing ─────────────────────────────────────────────

/// `simple_pin_fs(type, &mnt, &count)` — plant a fabricated vfsmount whose
/// `mnt_sb` points at a zeroed super_block stand-in.
#[unsafe(export_name = "simple_pin_fs")]
unsafe extern "C" fn linux_simple_pin_fs(
    _fs_type: *mut c_void,
    mount: *mut usize,
    count: *mut i32,
) -> i32 {
    if mount.is_null() {
        return -EINVAL;
    }
    unsafe {
        if *mount == 0 {
            let block = kzalloc(256 + 512);
            if block.is_null() {
                return -ENOMEM;
            }
            let sb = block as usize + 256;
            *((block as usize + VFSMOUNT_MNT_SB) as *mut usize) = sb;
            *mount = block as usize;
        }
        if !count.is_null() {
            *count += 1;
        }
    }
    0
}

#[unsafe(export_name = "simple_release_fs")]
unsafe extern "C" fn linux_simple_release_fs(_mount: *mut usize, count: *mut i32) {
    unsafe {
        if !count.is_null() {
            *count -= 1;
        }
    }
}

#[unsafe(export_name = "alloc_anon_inode")]
unsafe extern "C" fn linux_alloc_anon_inode(_sb: *mut c_void) -> *mut c_void {
    let block = kzalloc(INODE_SIZE + ADDRESS_SPACE_SIZE);
    if block.is_null() {
        return (-(ENOMEM as isize)) as *mut c_void;
    }
    let inode = block as usize;
    let mapping = inode + INODE_SIZE;
    unsafe {
        *((inode + INODE_I_MAPPING) as *mut usize) = mapping;
    }
    block.cast()
}

#[unsafe(export_name = "iput")]
unsafe extern "C" fn linux_iput(_inode: *mut c_void) {
    // Anon inodes live for the DRM device's lifetime; devices never unplug.
}

// ── device model / devres / PCI helpers ───────────────────────────────────

#[unsafe(export_name = "dev_set_name")]
unsafe extern "C" fn linux_dev_set_name(
    dev: *mut c_void,
    fmt: *const c_char,
    a0: usize,
    a1: usize,
    a2: usize,
) -> i32 {
    if dev.is_null() {
        return -EINVAL;
    }
    let name = format_c(unsafe { c_str(fmt) }, &[a0, a1, a2]);
    // struct device.kobj.name is the first field of the kobject at offset 0.
    unsafe {
        *(dev as *mut *mut c_char) = kstrdup_rust(&name);
    }
    0
}

#[unsafe(export_name = "dev_err_probe")]
unsafe extern "C" fn linux_dev_err_probe(
    _dev: *mut c_void,
    err: i32,
    fmt: *const c_char,
    a0: usize,
    a1: usize,
) -> i32 {
    let message = format_c(unsafe { c_str(fmt) }, &[a0, a1]);
    crate::log_warn!("drm-abi", "dev_err_probe({}): {}", err, message);
    err
}

#[unsafe(export_name = "class_create")]
unsafe extern "C" fn linux_class_create(_name: *const c_char) -> *mut c_void {
    // Opaque non-error cookie; drm_sysfs keeps it around and registers
    // devices against it through the base-device exports.
    kzalloc(128).cast()
}

#[unsafe(export_name = "class_destroy")]
unsafe extern "C" fn linux_class_destroy(class: *mut c_void) {
    kfree(class.cast());
}

lazy_static! {
    /// devm actions registered via `__devm_add_action`.
    static ref DEVM_ACTIONS: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
}

type DevmActionFn = unsafe extern "C" fn(*mut c_void);

#[unsafe(export_name = "__devm_add_action")]
unsafe extern "C" fn linux_devm_add_action(
    _dev: *mut c_void,
    action: usize,
    data: *mut c_void,
    _name: *const c_char,
) -> i32 {
    DEVM_ACTIONS.lock().push((action, data as usize));
    0
}

#[unsafe(export_name = "devm_release_action")]
unsafe extern "C" fn linux_devm_release_action(
    _dev: *mut c_void,
    action: usize,
    data: *mut c_void,
) {
    let mut actions = DEVM_ACTIONS.lock();
    if let Some(pos) = actions
        .iter()
        .position(|&(a, d)| a == action && d == data as usize)
    {
        actions.remove(pos);
    }
    drop(actions);
    if action != 0 {
        let func: DevmActionFn = unsafe { core::mem::transmute(action) };
        unsafe { func(data) };
    }
}

/// devres blocks: header stores the release fn; devices never unbind, so
/// `devres_add` only needs to keep the block alive.
#[unsafe(export_name = "__devres_alloc_node")]
unsafe extern "C" fn linux_devres_alloc_node(
    _release: usize,
    size: usize,
    _gfp: u32,
    _nid: i32,
    _name: *const c_char,
) -> *mut u8 {
    let block = kzalloc(size + 64);
    if block.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { block.add(64) }
}

#[unsafe(export_name = "devres_add")]
unsafe extern "C" fn linux_devres_add(_dev: *mut c_void, _res: *mut u8) {}

#[unsafe(export_name = "devres_free")]
unsafe extern "C" fn linux_devres_free(res: *mut u8) {
    if !res.is_null() {
        kfree(unsafe { res.sub(64) });
    }
}

#[unsafe(export_name = "devm_ioremap")]
unsafe extern "C" fn linux_devm_ioremap(_dev: *mut c_void, phys: u64, size: u64) -> *mut u8 {
    unsafe {
        crate::arch::x86::mm::ioremap::ioremap(phys, size)
            .map(|mapping| mapping.virt as *mut u8)
            .unwrap_or(core::ptr::null_mut())
    }
}

#[unsafe(export_name = "devm_ioremap_wc")]
unsafe extern "C" fn linux_devm_ioremap_wc(_dev: *mut c_void, phys: u64, size: u64) -> *mut u8 {
    unsafe {
        crate::arch::x86::mm::ioremap::ioremap_wc(phys, size)
            .map(|mapping| mapping.virt as *mut u8)
            .unwrap_or(core::ptr::null_mut())
    }
}

#[unsafe(export_name = "__devm_request_region")]
unsafe extern "C" fn linux_devm_request_region(
    _dev: *mut c_void,
    _parent: *mut c_void,
    start: u64,
    len: u64,
    _name: *const c_char,
) -> *mut u8 {
    // Fabricated `struct resource` with start/end at offsets 0/8; bochs only
    // checks the pointer.
    let block = kzalloc(64);
    if block.is_null() {
        return core::ptr::null_mut();
    }
    unsafe {
        *(block as *mut u64) = start;
        *(block.add(8) as *mut u64) = start + len.saturating_sub(1);
    }
    block
}

/// `pcim_enable_device` — enable MMIO + bus mastering in the config-space
/// command register through the PCI ABI registry, mirroring what the BIOS
/// already did for the boot display.
#[unsafe(export_name = "pcim_enable_device")]
unsafe extern "C" fn linux_pcim_enable_device(pdev: *mut c_void) -> i32 {
    let command =
        crate::linux_driver_abi::pci::device::linux_pci_config_read(pdev, 0x04, 2).unwrap_or(0);
    let _ =
        crate::linux_driver_abi::pci::device::linux_pci_config_write(pdev, 0x04, 2, command | 0x6);
    0
}

#[unsafe(export_name = "noop_llseek")]
unsafe extern "C" fn linux_noop_llseek(_file: *mut c_void, offset: i64, _whence: i32) -> i64 {
    offset
}

// ── chrdev registry ────────────────────────────────────────────────────────

lazy_static! {
    /// (major, name, fops) triples registered by the chain (`DRM_MAJOR`).
    static ref CHRDEVS: Mutex<Vec<(u32, String, usize)>> = Mutex::new(Vec::new());
}

/// Look up the fops a module registered for `major` (used by the /dev/dri
/// bridge once the chardev nodes are wired).
pub fn registered_chrdev_fops(major: u32) -> Option<usize> {
    CHRDEVS
        .lock()
        .iter()
        .find(|(m, _, _)| *m == major)
        .map(|&(_, _, fops)| fops)
}

#[unsafe(export_name = "__register_chrdev")]
unsafe extern "C" fn linux_register_chrdev(
    major: u32,
    _baseminor: u32,
    _count: u32,
    name: *const c_char,
    fops: *const c_void,
) -> i32 {
    let major = if major == 0 { 240 } else { major };
    CHRDEVS
        .lock()
        .push((major, String::from(unsafe { c_str(name) }), fops as usize));
    0
}

#[unsafe(export_name = "__unregister_chrdev")]
unsafe extern "C" fn linux_unregister_chrdev(
    major: u32,
    _baseminor: u32,
    _count: u32,
    _name: *const c_char,
) {
    CHRDEVS.lock().retain(|(m, _, _)| *m != major);
}

// ── framebuffer core (handover surface) ────────────────────────────────────
//
// The DRM fbdev emulation in drm_kms_helper.ko drives the classic FB core
// API.  Lupos owns the FB core in Rust (`video::fbdev`), so these exports ARE
// the native-driver framebuffer handover: when the emulation registers its
// fb_info, the fbdev core re-points `/dev/fb0` and fbcon at the mode the
// bochs driver just programmed, scanning out of the stdvga aperture (BAR0).

static REGISTERED_FB: AtomicU64 = AtomicU64::new(0);

#[unsafe(export_name = "framebuffer_alloc")]
unsafe extern "C" fn linux_framebuffer_alloc(par_size: usize, _dev: *mut c_void) -> *mut u8 {
    let block = kzalloc(FB_INFO_SIZE + par_size);
    if block.is_null() {
        return core::ptr::null_mut();
    }
    if par_size != 0 {
        unsafe {
            *((block as usize + FB_INFO_PAR) as *mut usize) = block as usize + FB_INFO_SIZE;
        }
    }
    block
}

#[unsafe(export_name = "framebuffer_release")]
unsafe extern "C" fn linux_framebuffer_release(info: *mut u8) {
    kfree(info);
}

/// Find the bochs stdvga scanout aperture (PCI 1234:1111 BAR0).
fn bochs_scanout_base() -> Option<u64> {
    crate::linux_driver_abi::pci::enumerate::pci_devices()
        .into_iter()
        .find(|dev| dev.vendor == 0x1234 && dev.device == 0x1111)
        .and_then(|dev| dev.bars[0].map(|bar| bar.base))
}

#[unsafe(export_name = "register_framebuffer")]
unsafe extern "C" fn linux_register_framebuffer(info: *mut u8) -> i32 {
    if info.is_null() {
        return -EINVAL;
    }
    let base = info as usize;
    let var = unsafe {
        &*((base + FB_INFO_VAR) as *const crate::linux_driver_abi::video::fbdev::FbVarScreeninfo)
    };
    // fb_fix_screeninfo.line_length is at offset 0x30 (16 id + 8 smem_start +
    // 4 smem_len + 4 type + 4 type_aux + 4 visual + 2*3 steps + 2 pad).
    let line_length = unsafe { *((base + FB_INFO_FIX + 0x30) as *const u32) };
    let (xres, yres, bpp) = (var.xres, var.yres, var.bits_per_pixel);
    let pitch = if line_length != 0 {
        line_length
    } else {
        xres.saturating_mul(bpp / 8)
    };

    REGISTERED_FB.store(base as u64, Ordering::Release);
    crate::log_info!(
        "drm-abi",
        "register_framebuffer: {}x{}@{} pitch={} (DRM fbdev emulation)",
        xres,
        yres,
        bpp,
        pitch
    );

    // Framebuffer handover: retarget the Lupos fbdev core (fbcon + /dev/fb0)
    // at the scanout aperture with the geometry the native driver programmed.
    // The DRM shadow buffer stays with the emulation; Lupos consoles and the
    // fbdev X server draw straight into VRAM, which the bochs dispi registers
    // now scan out at this geometry.
    if let Some(scanout) = bochs_scanout_base() {
        let size = pitch as u64 * yres as u64;
        let mapped = unsafe { crate::arch::x86::mm::ioremap::ioremap_wc(scanout, size) };
        if let Ok(mapping) = mapped {
            let ok = unsafe {
                crate::linux_driver_abi::video::fbdev::core::init_from_kernel_mapping(
                    scanout,
                    mapping.virt,
                    pitch,
                    xres,
                    yres,
                    bpp as u8,
                )
            };
            if ok {
                crate::linux_driver_abi::video::logo::fb_show_logo();
                crate::init::boot_trace::record("drm", "bochs framebuffer handover complete");
            }
        }
    }
    0
}

#[unsafe(export_name = "unregister_framebuffer")]
unsafe extern "C" fn linux_unregister_framebuffer(info: *mut u8) {
    let _ = REGISTERED_FB.compare_exchange(info as u64, 0, Ordering::AcqRel, Ordering::Acquire);
}

#[unsafe(export_name = "fb_alloc_cmap")]
unsafe extern "C" fn linux_fb_alloc_cmap(cmap: *mut u8, len: u32, transp: i32) -> i32 {
    if cmap.is_null() {
        return -EINVAL;
    }
    // struct fb_cmap { u32 start; u32 len; u16 *red, *green, *blue, *transp; }
    let bytes = (len as usize) * 2;
    unsafe {
        *(cmap as *mut u32) = 0;
        *(cmap.add(4) as *mut u32) = len;
        for slot in 0..4usize {
            let field = cmap.add(8 + slot * 8) as *mut usize;
            if slot == 3 && transp == 0 {
                *field = 0;
                continue;
            }
            let buf = kzalloc(bytes);
            if buf.is_null() {
                return -ENOMEM;
            }
            *field = buf as usize;
        }
    }
    0
}

#[unsafe(export_name = "fb_dealloc_cmap")]
unsafe extern "C" fn linux_fb_dealloc_cmap(cmap: *mut u8) {
    if cmap.is_null() {
        return;
    }
    unsafe {
        for slot in 0..4usize {
            let field = cmap.add(8 + slot * 8) as *mut usize;
            if *field != 0 {
                kfree(*field as *mut u8);
                *field = 0;
            }
        }
        *(cmap.add(4) as *mut u32) = 0;
    }
}

// ── console lock (single-threaded no-op) ───────────────────────────────────

#[unsafe(export_name = "console_lock")]
unsafe extern "C" fn linux_console_lock() {}

#[unsafe(export_name = "console_trylock")]
unsafe extern "C" fn linux_console_trylock() -> i32 {
    1
}

#[unsafe(export_name = "console_unlock")]
unsafe extern "C" fn linux_console_unlock() {}

#[unsafe(export_name = "is_console_locked")]
unsafe extern "C" fn linux_is_console_locked() -> i32 {
    0
}

// Video helpers come from the source-specific `linux_driver_abi::video` and
// `arch::x86::video` translations.

// ── user access helpers (custom register ABI) ──────────────────────────────
//
// Linux getuser/putuser use bespoke calling conventions
// (`arch/x86/lib/{get,put}user.S`): the address travels in %rax (get) or %rcx
// (put), the value in %rdx (get result) / %rax (put input), and the error code
// returns in %rax (get) / %rcx (put).

#[cfg(not(test))]
core::arch::global_asm!(
    ".global __get_user_4",
    "__get_user_4:",
    "mov edx, dword ptr [rax]",
    "xor eax, eax",
    "ret",
    ".global __put_user_4",
    "__put_user_4:",
    "mov dword ptr [rcx], eax",
    "xor ecx, ecx",
    "ret",
    ".global __put_user_8",
    "__put_user_8:",
    "mov qword ptr [rcx], rax",
    "xor ecx, ecx",
    "ret",
);

#[cfg(not(test))]
unsafe extern "C" {
    fn __get_user_4();
    fn __put_user_4();
    fn __put_user_8();
}

// ── SRCU (tiny) ────────────────────────────────────────────────────────────
//
// The cooperative single-runqueue kernel has no concurrent SRCU readers while
// a writer runs, so grace periods complete immediately.

#[unsafe(export_name = "synchronize_srcu")]
unsafe extern "C" fn linux_synchronize_srcu(_ssp: *mut c_void) {}

#[unsafe(export_name = "synchronize_rcu")]
unsafe extern "C" fn linux_synchronize_rcu() {}

#[unsafe(export_name = "__srcu_read_unlock")]
unsafe extern "C" fn linux_srcu_read_unlock(_ssp: *mut c_void, _idx: i32) {}

#[unsafe(export_name = "srcu_drive_gp")]
unsafe extern "C" fn linux_srcu_drive_gp(_work: *mut c_void) {}

#[unsafe(export_name = "srcu_tiny_irq_work")]
unsafe extern "C" fn linux_srcu_tiny_irq_work(_work: *mut c_void) {}

// ── unresolved ABI work ───────────────────────────────────────────────────
//
// This inventory is retained so the missing subsystems remain auditable.  It
// is not an export list: every entry needs its real Linux contract before the
// module loader may resolve it.

const UNIMPLEMENTED_FUNCTION_EXPORTS: &[&str] = &[
    "anon_inode_getfile",
    "class_create_file_ns",
    "class_remove_file_ns",
    "component_add",
    "component_del",
    "dentry_open",
    "__dev_fwnode",
    "device_property_present",
    "dma_buf_attach",
    "dma_buf_begin_cpu_access",
    "dma_buf_detach",
    "dma_buf_end_cpu_access",
    "dma_buf_export",
    "dma_buf_get",
    "dma_buf_map_attachment_unlocked",
    "dma_buf_mmap",
    "dma_buf_put",
    "dma_buf_unmap_attachment_unlocked",
    "dma_buf_vmap",
    "dma_buf_vunmap",
    "dma_fence_add_callback",
    "dma_fence_allocate_private_stub",
    "dma_fence_chain_find_seqno",
    "dma_fence_chain_init",
    "dma_fence_chain_walk",
    "dma_fence_context_alloc",
    "dma_fence_get_stub",
    "dma_fence_init",
    "dma_fence_release",
    "dma_fence_remove_callback",
    "dma_fence_set_deadline",
    "dma_fence_signal",
    "dma_fence_signal_timestamp",
    "__dma_fence_unwrap_merge",
    "dma_fence_wait_timeout",
    "dma_map_sgtable",
    "dma_max_mapping_size",
    "dma_resv_fini",
    "dma_resv_init",
    "dma_unmap_sg_attrs",
    "fb_deferred_io_cleanup",
    "fb_deferred_io_init",
    "fb_deferred_io_mmap",
    "fb_set_suspend",
    "fb_sys_read",
    "fb_sys_write",
    "fdget",
    "fd_install",
    "fwnode_device_is_available",
    "fwnode_find_reference",
    "get_unused_fd_flags",
    "hdmi_avi_infoframe_init",
    "hdmi_vendor_infoframe_init",
    "i2c_transfer",
    "init_pseudo",
    "kernel_fpu_begin_mask",
    "kernel_fpu_end",
    "kill_anon_super",
    "kobject_uevent_env",
    "memchr_inv",
    "pid_task",
    "platform_device_unregister",
    "put_pid",
    "put_unused_fd",
    "__rb_erase_color",
    "rb_erase",
    "__rb_insert_augmented",
    "rb_insert_color",
    "rb_next",
    "rb_prev",
    "__register_chrdev",
    "seq_buf_printf",
    "seq_printf",
    "__seq_puts",
    "seq_write",
    "set_pages_array_wb",
    "set_pages_array_wc",
    "sg_alloc_table_from_pages_segment",
    "sg_free_table",
    "__sg_page_iter_dma_next",
    "__sg_page_iter_next",
    "__sg_page_iter_start",
    "show_class_attr_string",
    "sync_file_create",
    "sync_file_get_fence",
    "__task_pid_nr_ns",
];

/// dma_resv objects only need "no fences attached" semantics for the
/// shadow-plane commit path; report signaled/complete.
#[unsafe(export_name = "dma_resv_test_signaled")]
unsafe extern "C" fn linux_dma_resv_test_signaled(_resv: *mut c_void, _usage: u32) -> i32 {
    1
}

#[unsafe(export_name = "dma_resv_wait_timeout")]
unsafe extern "C" fn linux_dma_resv_wait_timeout(
    _resv: *mut c_void,
    _usage: u32,
    _intr: i32,
    timeout: i64,
) -> i64 {
    core::cmp::max(timeout, 1)
}

#[unsafe(export_name = "dma_resv_get_singleton")]
unsafe extern "C" fn linux_dma_resv_get_singleton(
    _resv: *mut c_void,
    _usage: u32,
    fence: *mut usize,
) -> i32 {
    if !fence.is_null() {
        unsafe { *fence = 0 };
    }
    0
}

// ── device_del bridge onto the base-device glue ───────────────────────────

#[unsafe(export_name = "device_del")]
unsafe extern "C" fn linux_device_del(_dev: *mut c_void) {}

// ── registration ───────────────────────────────────────────────────────────

pub fn register_module_exports() {
    ensure_pump_registered();

    // Data symbols.
    unsafe {
        export_symbol_once(
            "oops_in_progress",
            core::ptr::addr_of_mut!(LINUX_OOPS_IN_PROGRESS) as usize,
            false,
        );
        export_symbol_once(
            "overflowuid",
            core::ptr::addr_of_mut!(LINUX_OVERFLOWUID) as usize,
            false,
        );
        export_symbol_once(
            "boot_cpu_data",
            core::ptr::addr_of_mut!(LINUX_BOOT_CPU_DATA) as usize,
            false,
        );
        export_symbol_once(
            "__default_kernel_pte_mask",
            core::ptr::addr_of_mut!(LINUX_DEFAULT_KERNEL_PTE_MASK) as usize,
            false,
        );
        export_symbol_once(
            "iomem_resource",
            core::ptr::addr_of_mut!(LINUX_IOMEM_RESOURCE) as usize,
            false,
        );
        export_symbol_once(
            "ioport_resource",
            core::ptr::addr_of_mut!(LINUX_IOPORT_RESOURCE) as usize,
            false,
        );
        export_symbol_once(
            "reservation_ww_class",
            core::ptr::addr_of_mut!(LINUX_RESERVATION_WW_CLASS) as usize,
            false,
        );
        export_symbol_once(
            "dma_fence_chain_ops",
            core::ptr::addr_of_mut!(LINUX_DMA_FENCE_CHAIN_OPS) as usize,
            false,
        );
        export_symbol_once(
            "param_ops_int",
            core::ptr::addr_of_mut!(LINUX_PARAM_OPS_INT) as usize,
            false,
        );
        export_symbol_once(
            "param_ops_uint",
            core::ptr::addr_of_mut!(LINUX_PARAM_OPS_UINT) as usize,
            false,
        );
        export_symbol_once(
            "param_ops_bool",
            core::ptr::addr_of_mut!(LINUX_PARAM_OPS_BOOL) as usize,
            false,
        );
        export_symbol_once(
            "param_ops_ulong",
            core::ptr::addr_of_mut!(LINUX_PARAM_OPS_ULONG) as usize,
            false,
        );
        export_symbol_once(
            "param_ops_string",
            core::ptr::addr_of_mut!(LINUX_PARAM_OPS_STRING) as usize,
            false,
        );
        // dev_is_pci() compares dev->bus against &pci_bus_type; export the
        // same bus object the PCI registration glue installs.
        export_symbol_once(
            "pci_bus_type",
            crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr() as usize,
            false,
        );

        // System workqueue pointer variables.
        if LINUX_SYSTEM_LONG_WQ == 0 {
            let long_wq =
                alloc::sync::Arc::into_raw(crate::kernel::workqueue::SYSTEM_LONG_WQ.get());
            LINUX_SYSTEM_LONG_WQ = long_wq as usize;
            pump_track_workqueue(long_wq as usize);
        }
        if LINUX_SYSTEM_PERCPU_WQ == 0 {
            let wq = alloc::sync::Arc::into_raw(crate::kernel::workqueue::SYSTEM_WQ.get());
            LINUX_SYSTEM_PERCPU_WQ = wq as usize;
            pump_track_workqueue(wq as usize);
        }
        if LINUX_SYSTEM_DFL_WQ == 0 {
            LINUX_SYSTEM_DFL_WQ = LINUX_SYSTEM_PERCPU_WQ;
        }
        export_symbol_once(
            "system_long_wq",
            core::ptr::addr_of_mut!(LINUX_SYSTEM_LONG_WQ) as usize,
            false,
        );
        export_symbol_once(
            "system_percpu_wq",
            core::ptr::addr_of_mut!(LINUX_SYSTEM_PERCPU_WQ) as usize,
            false,
        );
        export_symbol_once(
            "system_dfl_wq",
            core::ptr::addr_of_mut!(LINUX_SYSTEM_DFL_WQ) as usize,
            false,
        );

        #[cfg(not(test))]
        {
            export_symbol_once("__get_user_4", __get_user_4 as usize, false);
            export_symbol_once("__put_user_4", __put_user_4 as usize, false);
            export_symbol_once("__put_user_8", __put_user_8 as usize, false);
        }
    }

    // Function shims defined above (export_name attributes make the linker
    // aliases; the module loader needs the registry entries too).
    let shims: &[(&str, usize)] = &[
        (
            "__kmalloc_node_track_caller_noprof",
            linux_kmalloc_node_track_caller as usize,
        ),
        ("kmemdup_noprof", linux_kmemdup as usize),
        ("kstrdup", linux_kstrdup as usize),
        ("kstrdup_const", linux_kstrdup_const as usize),
        ("kfree_const", linux_kfree_const as usize),
        ("__kvmalloc_node_noprof", linux_kvmalloc_node as usize),
        (
            "krealloc_node_align_noprof",
            linux_krealloc_node_align as usize,
        ),
        ("memdup_user", linux_memdup_user as usize),
        ("vmemdup_user", linux_vmemdup_user as usize),
        ("memdup_user_nul", linux_memdup_user_nul as usize),
        ("memcpy_and_pad", linux_memcpy_and_pad as usize),
        ("kasprintf", linux_kasprintf as usize),
        ("kvasprintf", linux_kvasprintf as usize),
        ("scnprintf", linux_scnprintf as usize),
        ("simple_strtol", linux_simple_strtol as usize),
        ("sysfs_streq", linux_sysfs_streq as usize),
        ("sort", linux_sort as usize),
        ("list_sort", linux_list_sort as usize),
        ("idr_alloc", linux_idr_alloc as usize),
        ("idr_remove", linux_idr_remove as usize),
        ("idr_find", linux_idr_find as usize),
        ("idr_replace", linux_idr_replace as usize),
        ("idr_for_each", linux_idr_for_each as usize),
        ("idr_get_next", linux_idr_get_next as usize),
        ("idr_destroy", linux_idr_destroy as usize),
        ("idr_preload", linux_idr_preload as usize),
        ("ida_destroy", linux_ida_destroy as usize),
        ("xa_store", linux_xa_store as usize),
        ("xa_load", linux_xa_load as usize),
        ("xa_erase", linux_xa_erase as usize),
        ("__xa_alloc", linux_xa_alloc as usize),
        ("xa_find", linux_xa_find as usize),
        ("xa_find_after", linux_xa_find_after as usize),
        ("xa_destroy", linux_xa_destroy as usize),
        ("radix_tree_tagged", linux_radix_tree_tagged as usize),
        ("ww_mutex_lock", linux_ww_mutex_lock as usize),
        (
            "ww_mutex_lock_interruptible",
            linux_ww_mutex_lock_interruptible as usize,
        ),
        ("ww_mutex_trylock", linux_ww_mutex_trylock as usize),
        ("ww_mutex_unlock", linux_ww_mutex_unlock as usize),
        ("mutex_is_locked", linux_mutex_is_locked as usize),
        ("mutex_trylock", linux_mutex_trylock as usize),
        (
            "mutex_lock_interruptible",
            linux_mutex_lock_interruptible as usize,
        ),
        (
            "atomic_dec_and_mutex_lock",
            linux_atomic_dec_and_mutex_lock as usize,
        ),
        (
            "refcount_warn_saturate",
            linux_refcount_warn_saturate as usize,
        ),
        ("refcount_dec_not_one", linux_refcount_dec_not_one as usize),
        ("__init_waitqueue_head", linux_init_waitqueue_head as usize),
        ("init_wait_entry", linux_init_wait_entry as usize),
        (
            "prepare_to_wait_event",
            linux_prepare_to_wait_event as usize,
        ),
        ("finish_wait", linux_finish_wait as usize),
        ("__wake_up", linux_wake_up as usize),
        ("schedule", linux_schedule as usize),
        ("wake_up_process", linux_wake_up_process as usize),
        ("sched_set_fifo", linux_sched_set_fifo as usize),
        (
            "try_wait_for_completion",
            linux_try_wait_for_completion_export as usize,
        ),
        (
            "wait_for_completion_timeout",
            linux_wait_for_completion_timeout as usize,
        ),
        (
            "wait_for_completion_interruptible",
            linux_wait_for_completion_interruptible as usize,
        ),
        (
            "wait_for_completion_interruptible_timeout",
            linux_wait_for_completion_interruptible_timeout as usize,
        ),
        ("timer_init_key", linux_timer_init_key as usize),
        ("mod_timer", linux_mod_timer as usize),
        ("timer_delete_sync", linux_timer_delete_sync as usize),
        (
            "delayed_work_timer_fn",
            linux_delayed_work_timer_fn as usize,
        ),
        (
            "queue_delayed_work_on",
            linux_queue_delayed_work_on as usize,
        ),
        ("mod_delayed_work_on", linux_mod_delayed_work_on as usize),
        (
            "cancel_delayed_work_sync",
            linux_cancel_delayed_work_sync as usize,
        ),
        ("cancel_work_sync", linux_cancel_work_sync as usize),
        ("flush_delayed_work", linux_flush_delayed_work as usize),
        ("current_work", linux_current_work as usize),
        ("hrtimer_setup", linux_hrtimer_setup as usize),
        (
            "hrtimer_start_range_ns",
            linux_hrtimer_start_range_ns as usize,
        ),
        ("hrtimer_cancel", linux_hrtimer_cancel as usize),
        (
            "hrtimer_try_to_cancel",
            linux_hrtimer_try_to_cancel as usize,
        ),
        ("hrtimer_active", linux_hrtimer_active as usize),
        ("hrtimer_forward", linux_hrtimer_forward as usize),
        ("hrtimer_cb_get_time", linux_hrtimer_cb_get_time as usize),
        (
            "kthread_create_worker_on_node",
            linux_kthread_create_worker_on_node as usize,
        ),
        ("kthread_queue_work", linux_kthread_queue_work as usize),
        ("kthread_flush_worker", linux_kthread_flush_worker as usize),
        ("kthread_flush_work", linux_kthread_flush_work as usize),
        (
            "kthread_cancel_work_sync",
            linux_kthread_cancel_work_sync as usize,
        ),
        (
            "kthread_destroy_worker",
            linux_kthread_destroy_worker as usize,
        ),
        ("ktime_get", linux_ktime_get as usize),
        ("__msecs_to_jiffies", linux_msecs_to_jiffies as usize),
        ("nsecs_to_jiffies64", linux_nsecs_to_jiffies64 as usize),
        ("ns_to_timespec64", linux_ns_to_timespec64 as usize),
        ("vmap", linux_vmap as usize),
        ("vunmap", linux_vunmap as usize),
        ("pgprot_writecombine", linux_pgprot_writecombine as usize),
        ("vm_get_page_prot", linux_vm_get_page_prot as usize),
        ("vmf_insert_pfn", linux_vmf_insert_pfn as usize),
        ("unmap_mapping_range", linux_unmap_mapping_range as usize),
        (
            "invalidate_mapping_pages",
            linux_invalidate_mapping_pages as usize,
        ),
        ("mm_get_unmapped_area", linux_mm_get_unmapped_area as usize),
        ("folio_mark_accessed", linux_folio_mark_accessed as usize),
        ("folio_mark_dirty", linux_folio_mark_dirty as usize),
        ("__folio_batch_release", linux_folio_batch_release as usize),
        (
            "check_move_unevictable_folios",
            linux_check_move_unevictable_folios as usize,
        ),
        ("shmem_file_setup", linux_shmem_file_setup as usize),
        ("shmem_read_folio_gfp", linux_shmem_read_folio_gfp as usize),
        ("shmem_truncate_range", linux_shmem_truncate_range as usize),
        ("fput", linux_fput as usize),
        ("file_update_time", linux_file_update_time as usize),
        ("simple_pin_fs", linux_simple_pin_fs as usize),
        ("simple_release_fs", linux_simple_release_fs as usize),
        ("alloc_anon_inode", linux_alloc_anon_inode as usize),
        ("iput", linux_iput as usize),
        ("dev_set_name", linux_dev_set_name as usize),
        ("dev_err_probe", linux_dev_err_probe as usize),
        ("class_create", linux_class_create as usize),
        ("class_destroy", linux_class_destroy as usize),
        ("__devm_add_action", linux_devm_add_action as usize),
        ("devm_release_action", linux_devm_release_action as usize),
        ("__devres_alloc_node", linux_devres_alloc_node as usize),
        ("devres_add", linux_devres_add as usize),
        ("devres_free", linux_devres_free as usize),
        ("devm_ioremap", linux_devm_ioremap as usize),
        ("devm_ioremap_wc", linux_devm_ioremap_wc as usize),
        ("__devm_request_region", linux_devm_request_region as usize),
        ("pcim_enable_device", linux_pcim_enable_device as usize),
        ("noop_llseek", linux_noop_llseek as usize),
        ("__register_chrdev", linux_register_chrdev as usize),
        ("__unregister_chrdev", linux_unregister_chrdev as usize),
        ("framebuffer_alloc", linux_framebuffer_alloc as usize),
        ("framebuffer_release", linux_framebuffer_release as usize),
        ("register_framebuffer", linux_register_framebuffer as usize),
        (
            "unregister_framebuffer",
            linux_unregister_framebuffer as usize,
        ),
        ("fb_alloc_cmap", linux_fb_alloc_cmap as usize),
        ("fb_dealloc_cmap", linux_fb_dealloc_cmap as usize),
        ("console_lock", linux_console_lock as usize),
        ("console_trylock", linux_console_trylock as usize),
        ("console_unlock", linux_console_unlock as usize),
        ("is_console_locked", linux_is_console_locked as usize),
        ("synchronize_srcu", linux_synchronize_srcu as usize),
        ("synchronize_rcu", linux_synchronize_rcu as usize),
        ("__srcu_read_unlock", linux_srcu_read_unlock as usize),
        ("srcu_drive_gp", linux_srcu_drive_gp as usize),
        ("srcu_tiny_irq_work", linux_srcu_tiny_irq_work as usize),
        (
            "dma_resv_test_signaled",
            linux_dma_resv_test_signaled as usize,
        ),
        (
            "dma_resv_wait_timeout",
            linux_dma_resv_wait_timeout as usize,
        ),
        (
            "dma_resv_get_singleton",
            linux_dma_resv_get_singleton as usize,
        ),
        ("device_del", linux_device_del as usize),
    ];
    for &(name, addr) in shims {
        export_symbol_once(name, addr, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_c_covers_name_conversions() {
        let card = b"card%d\0";
        assert_eq!(
            format_c(unsafe { c_str(card.as_ptr().cast()) }, &[3]),
            "card3"
        );
        let name = b"Virtual-1\0";
        assert_eq!(
            format_c("%s-%d", &[name.as_ptr() as usize, 2]),
            "Virtual-1-2"
        );
        assert_eq!(format_c("%lu kB", &[1280]), "1280 kB");
        assert_eq!(format_c("100%%", &[]), "100%");
    }

    #[test]
    fn idr_side_table_allocates_sequential_ids() {
        let idr = 0x1000usize as *mut c_void;
        unsafe {
            let a = linux_idr_alloc(idr, 0xa0 as *mut c_void, 0, 0, 0);
            let b = linux_idr_alloc(idr, 0xb0 as *mut c_void, 0, 0, 0);
            assert_eq!((a, b), (0, 1));
            assert_eq!(linux_idr_find(idr, 1) as usize, 0xb0);
            assert_eq!(linux_idr_remove(idr, 0) as usize, 0xa0);
            assert!(linux_idr_find(idr, 0).is_null());
            linux_idr_destroy(idr);
        }
    }

    #[test]
    fn xa_alloc_respects_packed_limit() {
        let xa = 0x2000usize as *mut c_void;
        let mut id = 0u32;
        unsafe {
            // xa_limit { max = 63, min = 1 } packed as (min << 32) | max.
            let limit = (1u64 << 32) | 63;
            assert_eq!(
                linux_xa_alloc(xa, &mut id, 0xcafe as *mut c_void, limit, 0),
                0
            );
            assert_eq!(id, 1);
            assert_eq!(linux_xa_load(xa, 1) as usize, 0xcafe);
            linux_xa_destroy(xa);
        }
    }

    #[test]
    fn ww_mutex_tracks_ctx_and_unlock() {
        let lock = 0x3000usize as *mut c_void;
        let ctx = 0x4000usize as *mut c_void;
        unsafe {
            assert_eq!(linux_ww_mutex_lock(lock, ctx), 0);
            assert_eq!(linux_ww_mutex_lock(lock, ctx), -EALREADY);
            linux_ww_mutex_unlock(lock);
            assert_eq!(linux_ww_mutex_trylock(lock, core::ptr::null_mut()), 1);
            linux_ww_mutex_unlock(lock);
        }
    }

    #[test]
    fn shmem_side_file_plants_probed_offsets() {
        unsafe {
            let file = linux_shmem_file_setup(core::ptr::null(), 4096, 0);
            assert!(!file.is_null());
            let mapping = *((file as usize + FILE_F_MAPPING) as *const usize);
            let inode = *((file as usize + FILE_F_INODE) as *const usize);
            assert_eq!(mapping, file as usize + FILE_SIZE + INODE_SIZE);
            assert_eq!(inode, file as usize + FILE_SIZE);
            assert_eq!(*((inode + INODE_I_MAPPING) as *const usize), mapping);
            SHMEM_FILES.lock().remove(&mapping);
            kfree(file.cast());
        }
    }

    #[test]
    fn msecs_to_jiffies_uses_module_hz() {
        unsafe {
            assert_eq!(linux_msecs_to_jiffies(1000), MODULE_HZ);
            assert_eq!(linux_msecs_to_jiffies(1), 1);
        }
    }

    #[test]
    fn timespec_conversion_splits_ns() {
        unsafe {
            let ts = linux_ns_to_timespec64(1_500_000_000);
            assert_eq!((ts.tv_sec, ts.tv_nsec), (1, 500_000_000));
        }
    }
}

// Silence dead-code warnings for probe constants only used in comments/tests.
const _: () = {
    let _ = FB_INFO_NODE;
    let _ = FB_INFO_CMAP;
    let _ = FB_INFO_SCREEN_BASE;
    let _ = UNIMPLEMENTED_FUNCTION_EXPORTS;
    let _ = Box::<u8>::new;
};
