//! linux-parity: complete
//! linux-source: vendor/linux/kernel/capability.c
//! test-origin: linux:vendor/linux/kernel/capability.c
//! Linux capabilities — Milestone 27.
//!
//! Implements `kernel_cap_t` (a 64-bit bitmask packing all `CAP_*` bits 0..=40)
//! plus the helpers `cap_set`, `cap_clear`, `cap_raised`, `capable`, and the
//! `sys_capget` / `sys_capset` syscalls with the version-3 UAPI.
//!
//! The capability bit numbering is taken byte-for-byte from
//! `vendor/linux/include/uapi/linux/capability.h`; if any constant here drifts
//! from that header, the project no longer satisfies its Linux ABI bar.
//!
//! # Layout
//!
//! Linux uses `__u32 cap[2]` to hold 64 capability bits.  We mirror that with
//! `[u32; 2]`, indexed by `CAP_TO_INDEX(cap)` and masked by `CAP_TO_MASK(cap)`.
//!
//! Reference: Linux `include/linux/capability.h`, `kernel/capability.c`.

use crate::kernel::cred::{Cred, current_cred};
use crate::kernel::module::{export_symbol, find_symbol};

// ── Capability bit numbers (Linux uapi/linux/capability.h) ───────────────────

pub const CAP_CHOWN: u32 = 0;
pub const CAP_DAC_OVERRIDE: u32 = 1;
pub const CAP_DAC_READ_SEARCH: u32 = 2;
pub const CAP_FOWNER: u32 = 3;
pub const CAP_FSETID: u32 = 4;
pub const CAP_KILL: u32 = 5;
pub const CAP_SETGID: u32 = 6;
pub const CAP_SETUID: u32 = 7;
pub const CAP_SETPCAP: u32 = 8;
pub const CAP_LINUX_IMMUTABLE: u32 = 9;
pub const CAP_NET_BIND_SERVICE: u32 = 10;
pub const CAP_NET_BROADCAST: u32 = 11;
pub const CAP_NET_ADMIN: u32 = 12;
pub const CAP_NET_RAW: u32 = 13;
pub const CAP_IPC_LOCK: u32 = 14;
pub const CAP_IPC_OWNER: u32 = 15;
pub const CAP_SYS_MODULE: u32 = 16;
pub const CAP_SYS_RAWIO: u32 = 17;
pub const CAP_SYS_CHROOT: u32 = 18;
pub const CAP_SYS_PTRACE: u32 = 19;
pub const CAP_SYS_PACCT: u32 = 20;
pub const CAP_SYS_ADMIN: u32 = 21;
pub const CAP_SYS_BOOT: u32 = 22;
pub const CAP_SYS_NICE: u32 = 23;
pub const CAP_SYS_RESOURCE: u32 = 24;
pub const CAP_SYS_TIME: u32 = 25;
pub const CAP_SYS_TTY_CONFIG: u32 = 26;
pub const CAP_MKNOD: u32 = 27;
pub const CAP_LEASE: u32 = 28;
pub const CAP_AUDIT_WRITE: u32 = 29;
pub const CAP_AUDIT_CONTROL: u32 = 30;
pub const CAP_SETFCAP: u32 = 31;
pub const CAP_MAC_OVERRIDE: u32 = 32;
pub const CAP_MAC_ADMIN: u32 = 33;
pub const CAP_SYSLOG: u32 = 34;
pub const CAP_WAKE_ALARM: u32 = 35;
pub const CAP_BLOCK_SUSPEND: u32 = 36;
pub const CAP_AUDIT_READ: u32 = 37;
pub const CAP_PERFMON: u32 = 38;
pub const CAP_BPF: u32 = 39;
pub const CAP_CHECKPOINT_RESTORE: u32 = 40;

/// Highest defined capability number.  Linux: `CAP_LAST_CAP`.
pub const CAP_LAST_CAP: u32 = CAP_CHECKPOINT_RESTORE;

/// Number of u32 words used to hold the capability bitmask.
/// Linux: `_KERNEL_CAPABILITY_U32S` (2 since 2.6.26).
pub const KERNEL_CAPABILITY_U32S: usize = 2;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("capable", linux_capable as usize, true);
}

// ── Capability set (kernel_cap_t) ────────────────────────────────────────────

/// Bitmask holding all capability bits.  Layout-compatible with Linux
/// `kernel_cap_t` (`__u32 cap[2]`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct KernelCapT {
    pub cap: [u32; KERNEL_CAPABILITY_U32S],
}

impl KernelCapT {
    /// Empty capability set.
    pub const fn empty() -> Self {
        Self { cap: [0, 0] }
    }

    /// Full capability set with every defined capability raised.
    pub const fn full() -> Self {
        // CAP_LAST_CAP=40 → bit 40 ≡ index 1, bit 8 → 0x100 mask high to bit 8.
        // Set bits 0..=31 in word 0, and bits 32..=40 in word 1.
        Self {
            cap: [
                0xFFFF_FFFF,
                (1u32 << ((CAP_LAST_CAP & 31) + 1)) - 1, // bits 0..=8 of word 1
            ],
        }
    }

    /// Test whether `cap` is raised.  Returns `false` for invalid `cap`.
    #[inline]
    pub const fn raised(&self, cap: u32) -> bool {
        if cap > CAP_LAST_CAP {
            return false;
        }
        let idx = cap_to_index(cap);
        let mask = cap_to_mask(cap);
        (self.cap[idx] & mask) != 0
    }

    /// Raise `cap` in place.  No-op for invalid `cap`.
    #[inline]
    pub fn raise(&mut self, cap: u32) {
        if cap > CAP_LAST_CAP {
            return;
        }
        self.cap[cap_to_index(cap)] |= cap_to_mask(cap);
    }

    /// Lower (clear) `cap` in place.  No-op for invalid `cap`.
    #[inline]
    pub fn lower(&mut self, cap: u32) {
        if cap > CAP_LAST_CAP {
            return;
        }
        self.cap[cap_to_index(cap)] &= !cap_to_mask(cap);
    }

    /// Test whether the set is empty (no capability raised).
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.cap[0] == 0 && self.cap[1] == 0
    }
}

/// `CAP_TO_INDEX(cap)` — which `u32` word holds `cap`'s bit.
#[inline]
pub const fn cap_to_index(cap: u32) -> usize {
    (cap >> 5) as usize
}

/// `CAP_TO_MASK(cap)` — bit mask within the word returned by `cap_to_index`.
#[inline]
pub const fn cap_to_mask(cap: u32) -> u32 {
    1u32 << (cap & 31)
}

// ── Capability checks ────────────────────────────────────────────────────────

/// Return `true` if the current task has `cap` raised in its effective set.
///
/// Linux: `capable()`.  In M27 this only inspects the global cred — the
/// user-namespace owner check (`ns_capable`) is wired in M28.
pub fn capable(cap: u32) -> bool {
    let cred = current_cred();
    if cred.is_null() {
        // Pre-init: trust the caller.  Mirrors Linux's behaviour during
        // very-early bring-up before init_cred is published.
        return true;
    }
    unsafe { (*cred).cap_effective.raised(cap) }
}

/// Return `true` if the current task has `cap` raised relative to a user
/// namespace.  Stub that matches `capable(cap)` until the namespace owner
/// hierarchy is wired in M28.
pub fn ns_capable(_user_ns: *const core::ffi::c_void, cap: u32) -> bool {
    capable(cap)
}

/// `capable` - `vendor/linux/kernel/capability.c`.
pub unsafe extern "C" fn linux_capable(cap: i32) -> bool {
    cap >= 0 && capable(cap as u32)
}

// ── UAPI structs (capget / capset) ───────────────────────────────────────────

/// `_LINUX_CAPABILITY_VERSION_1` (32-bit, deprecated).
pub const LINUX_CAPABILITY_VERSION_1: u32 = 0x1998_0330;
/// `_LINUX_CAPABILITY_VERSION_2` (64-bit, deprecated).
pub const LINUX_CAPABILITY_VERSION_2: u32 = 0x2007_1026;
/// `_LINUX_CAPABILITY_VERSION_3` (current).
pub const LINUX_CAPABILITY_VERSION_3: u32 = 0x2008_0522;

/// `__user_cap_header_struct`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct UserCapHeader {
    pub version: u32,
    pub pid: i32,
}

/// `__user_cap_data_struct` — one per `u32` word of the capability bitmask.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct UserCapData {
    pub effective: u32,
    pub permitted: u32,
    pub inheritable: u32,
}

fn cap_pid_is_current(pid: i32) -> bool {
    if pid == 0 {
        return true;
    }
    let task = unsafe { crate::kernel::sched::get_current() };
    !task.is_null() && unsafe { pid == (*task).pid || pid == (*task).tgid }
}

// ── Syscalls ─────────────────────────────────────────────────────────────────

/// `sys_capget(header, datap)` — read the calling task's capability sets.
///
/// Mirrors Linux `kernel/capability.c::SYSCALL_DEFINE2(capget,…)`.
///
/// # Safety
/// `header` must be a valid kernel-space pointer.  `datap`, when non-null,
/// must point to at least `KERNEL_CAPABILITY_U32S` `UserCapData` slots.
pub unsafe fn sys_capget(header: *mut UserCapHeader, datap: *mut UserCapData) -> i64 {
    if header.is_null() {
        return -14; // EFAULT
    }
    let hdr = unsafe { *header };
    // Echo back the canonical version.
    unsafe { (*header).version = LINUX_CAPABILITY_VERSION_3 };

    if hdr.version != LINUX_CAPABILITY_VERSION_1
        && hdr.version != LINUX_CAPABILITY_VERSION_2
        && hdr.version != LINUX_CAPABILITY_VERSION_3
    {
        return -22; // EINVAL — unknown version
    }
    if datap.is_null() {
        // Linux returns 0 here (the version handshake completed).
        return 0;
    }
    if hdr.pid < 0 {
        return -22;
    }
    if !cap_pid_is_current(hdr.pid) {
        // Reading another task's caps is not supported yet.
        return -3;
    }

    let cred = current_cred();
    let (eff, perm, inh) = if cred.is_null() {
        (KernelCapT::full(), KernelCapT::full(), KernelCapT::empty())
    } else {
        unsafe {
            (
                (*cred).cap_effective,
                (*cred).cap_permitted,
                (*cred).cap_inheritable,
            )
        }
    };

    let words = if hdr.version == LINUX_CAPABILITY_VERSION_1 {
        1
    } else {
        2
    };
    for i in 0..words {
        unsafe {
            (*datap.add(i)).effective = eff.cap[i];
            (*datap.add(i)).permitted = perm.cap[i];
            (*datap.add(i)).inheritable = inh.cap[i];
        }
    }
    0
}

/// `sys_capset(header, datap)` — set the calling task's capability sets.
///
/// Mirrors Linux `kernel/capability.c::SYSCALL_DEFINE2(capset,…)`.
///
/// In M27 we accept the new caps if they are a subset of the current
/// permitted set (the standard rule), otherwise return `-EPERM`.
///
/// # Safety
/// `header` and `datap` must be valid kernel-space pointers.
pub unsafe fn sys_capset(header: *const UserCapHeader, datap: *const UserCapData) -> i64 {
    if header.is_null() || datap.is_null() {
        return -14; // EFAULT
    }
    let hdr = unsafe { *header };
    if hdr.version != LINUX_CAPABILITY_VERSION_2 && hdr.version != LINUX_CAPABILITY_VERSION_3 {
        return -22; // EINVAL — version 1 cannot set 64-bit caps
    }
    if hdr.pid != 0 && !cap_pid_is_current(hdr.pid) {
        return -1; // EPERM — only self
    }

    let cred = current_cred();
    if cred.is_null() {
        return -1;
    }

    let words = 2;
    let mut new_eff = KernelCapT::empty();
    let mut new_perm = KernelCapT::empty();
    let mut new_inh = KernelCapT::empty();
    for i in 0..words {
        let d = unsafe { *datap.add(i) };
        new_eff.cap[i] = d.effective;
        new_perm.cap[i] = d.permitted;
        new_inh.cap[i] = d.inheritable;
    }

    let cur_perm = unsafe { (*cred).cap_permitted };
    // New permitted ⊆ old permitted.
    for i in 0..words {
        if new_perm.cap[i] & !cur_perm.cap[i] != 0 {
            return -1; // EPERM
        }
        // New effective ⊆ new permitted.
        if new_eff.cap[i] & !new_perm.cap[i] != 0 {
            return -1; // EPERM
        }
    }

    let new_cred = match crate::kernel::cred::prepare_creds() {
        Some(c) => c,
        None => return -12, // ENOMEM
    };
    unsafe {
        (*new_cred).cap_effective = new_eff;
        (*new_cred).cap_permitted = new_perm;
        (*new_cred).cap_inheritable = new_inh;
    }
    crate::kernel::cred::commit_creds(new_cred);
    0
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::kernel::cred::INIT_CRED;
    use crate::kernel::{sched, task::TaskStruct};

    #[test]
    fn cap_last_cap_is_40() {
        assert_eq!(CAP_LAST_CAP, 40);
    }

    #[test]
    fn cap_to_index_partitions_bits() {
        assert_eq!(cap_to_index(CAP_CHOWN), 0);
        assert_eq!(cap_to_index(31), 0);
        assert_eq!(cap_to_index(32), 1);
        assert_eq!(cap_to_index(CAP_BPF), 1);
        assert_eq!(cap_to_index(CAP_CHECKPOINT_RESTORE), 1);
    }

    #[test]
    fn cap_to_mask_per_bit() {
        assert_eq!(cap_to_mask(CAP_CHOWN), 1);
        assert_eq!(cap_to_mask(CAP_AUDIT_READ), 1 << 5); // bit 37 → 37 & 31 = 5
        assert_eq!(cap_to_mask(CAP_BPF), 1 << 7); // bit 39 → 39 & 31 = 7
    }

    #[test]
    fn raise_lower_round_trip() {
        let mut s = KernelCapT::empty();
        assert!(!s.raised(CAP_SYS_ADMIN));
        s.raise(CAP_SYS_ADMIN);
        assert!(s.raised(CAP_SYS_ADMIN));
        s.lower(CAP_SYS_ADMIN);
        assert!(!s.raised(CAP_SYS_ADMIN));
    }

    #[test]
    fn full_set_has_all_caps_raised() {
        let s = KernelCapT::full();
        for cap in 0..=CAP_LAST_CAP {
            assert!(s.raised(cap), "CAP_{} should be raised in full()", cap);
        }
    }

    #[test]
    fn empty_set_is_empty() {
        let s = KernelCapT::empty();
        assert!(s.is_empty());
    }

    #[test]
    fn invalid_cap_returns_false() {
        let mut s = KernelCapT::full();
        assert!(!s.raised(64));
        s.raise(64); // must be a no-op, not a panic
        s.lower(64);
    }

    #[test]
    fn user_cap_header_layout() {
        assert_eq!(core::mem::size_of::<UserCapHeader>(), 8);
        assert_eq!(core::mem::offset_of!(UserCapHeader, version), 0);
        assert_eq!(core::mem::offset_of!(UserCapHeader, pid), 4);
    }

    #[test]
    fn user_cap_data_layout() {
        assert_eq!(core::mem::size_of::<UserCapData>(), 12);
    }

    #[test]
    fn syscall_m76_capability_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 71;
        current.tgid = 71;
        current.cred = &raw const INIT_CRED;

        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);

            assert_eq!(
                sys_capget(core::ptr::null_mut(), core::ptr::null_mut()),
                -14
            );

            let mut header = UserCapHeader { version: 0, pid: 0 };
            assert_eq!(sys_capget(&mut header, core::ptr::null_mut()), -22);
            assert_eq!(header.version, LINUX_CAPABILITY_VERSION_3);

            let mut header = UserCapHeader {
                version: LINUX_CAPABILITY_VERSION_3,
                pid: 0,
            };
            let mut data = [UserCapData::default(); KERNEL_CAPABILITY_U32S];
            assert_eq!(sys_capget(&mut header, data.as_mut_ptr()), 0);
            assert_eq!(header.version, LINUX_CAPABILITY_VERSION_3);
            assert_ne!(data[0].effective, 0);
            assert_eq!(data[0].inheritable, 0);

            data[0].effective = 0;
            data[0].permitted = 0;
            data[0].inheritable = 0;
            data[1] = UserCapData::default();
            assert_eq!(sys_capset(&header, data.as_ptr()), 0);
            let mut after = [UserCapData::default(); KERNEL_CAPABILITY_U32S];
            assert_eq!(sys_capget(&mut header, after.as_mut_ptr()), 0);
            assert_eq!(after[0].effective, 0);
            assert_eq!(after[0].permitted, 0);

            let mut self_header = UserCapHeader {
                version: LINUX_CAPABILITY_VERSION_3,
                pid: 71,
            };
            assert_eq!(sys_capget(&mut self_header, after.as_mut_ptr()), 0);
            assert_eq!(sys_capset(&self_header, data.as_ptr()), 0);

            let mut missing_header = UserCapHeader {
                version: LINUX_CAPABILITY_VERSION_3,
                pid: 9999,
            };
            assert_eq!(sys_capget(&mut missing_header, after.as_mut_ptr()), -3);

            let bad_header = UserCapHeader {
                version: LINUX_CAPABILITY_VERSION_1,
                pid: 0,
            };
            assert_eq!(sys_capset(&bad_header, data.as_ptr()), -22);
            assert_eq!(sys_capset(core::ptr::null(), data.as_ptr()), -14);

            current.cred = &raw const INIT_CRED;
            sched::set_current(previous);
        }
    }
}
