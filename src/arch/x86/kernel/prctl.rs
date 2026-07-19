//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86-64 `arch_prctl(2)` support.
//!
//! M65 wires the Linux ABI operations that ordinary libc TLS setup needs:
//! `ARCH_SET_FS`, `ARCH_GET_FS`, `ARCH_SET_GS`, and `ARCH_GET_GS`.

extern crate alloc;

use crate::{arch::x86::kernel::uaccess, kernel::sched};

pub const ARCH_SET_GS: i32 = 0x1001;
pub const ARCH_SET_FS: i32 = 0x1002;
pub const ARCH_GET_FS: i32 = 0x1003;
pub const ARCH_GET_GS: i32 = 0x1004;

const EINVAL: i64 = -22;
const EFAULT: i64 = -14;

/// Linux `arch_prctl(code, addr)` for x86-64 TLS base management.
///
/// # Safety
/// For GET operations, `addr` must point to writable user memory; the value is
/// copied through the uaccess helpers so kernel addresses and bad user ranges
/// fail with `EFAULT`.
pub unsafe fn sys_arch_prctl(code: i32, addr: u64) -> i64 {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return EINVAL;
    }

    unsafe {
        match code {
            ARCH_SET_FS => {
                (*task).thread.fsbase = addr;
                (*task).thread.fsindex = 0;
                write_fs_base(addr);
                0
            }
            ARCH_SET_GS => {
                (*task).thread.gsbase = addr;
                (*task).thread.gsindex = 0;
                write_user_gs_base(addr);
                0
            }
            ARCH_GET_FS => {
                let base = read_fs_base((*task).thread.fsbase);
                (*task).thread.fsbase = base;
                copy_tls_base_to_user(addr, base)
            }
            ARCH_GET_GS => {
                let base = read_user_gs_base((*task).thread.gsbase);
                (*task).thread.gsbase = base;
                copy_tls_base_to_user(addr, base)
            }
            _ => EINVAL,
        }
    }
}

#[inline]
unsafe fn copy_tls_base_to_user(addr: u64, value: u64) -> i64 {
    if addr == 0 {
        return EFAULT;
    }

    let not_copied = unsafe {
        uaccess::copy_to_user(
            addr as *mut u8,
            &value as *const u64 as *const u8,
            core::mem::size_of::<u64>(),
        )
    };

    if not_copied == 0 { 0 } else { EFAULT }
}

#[cfg(not(test))]
const MSR_FS_BASE: u32 = 0xC000_0100;
#[cfg(not(test))]
const MSR_KERNEL_GS_BASE: u32 = 0xC000_0102;

#[cfg(not(test))]
#[inline]
unsafe fn write_fs_base(value: u64) {
    unsafe { wrmsr_raw(MSR_FS_BASE, value) };
}

#[cfg(test)]
#[inline]
unsafe fn write_fs_base(_value: u64) {}

#[cfg(not(test))]
#[inline]
unsafe fn write_user_gs_base(value: u64) {
    // SYSCALL entry has already executed SWAPGS, so the inactive user GS base
    // resides in IA32_KERNEL_GS_BASE until the return-to-user SWAPGS.
    unsafe { wrmsr_raw(MSR_KERNEL_GS_BASE, value) };
}

#[cfg(test)]
#[inline]
unsafe fn write_user_gs_base(_value: u64) {}

#[cfg(not(test))]
#[inline]
unsafe fn read_fs_base(_fallback: u64) -> u64 {
    unsafe { rdmsr_raw(MSR_FS_BASE) }
}

#[cfg(test)]
#[inline]
unsafe fn read_fs_base(fallback: u64) -> u64 {
    fallback
}

#[cfg(not(test))]
#[inline]
unsafe fn read_user_gs_base(_fallback: u64) -> u64 {
    // See write_user_gs_base(): user GS is inactive while this syscall runs.
    unsafe { rdmsr_raw(MSR_KERNEL_GS_BASE) }
}

#[cfg(test)]
#[inline]
unsafe fn read_user_gs_base(fallback: u64) -> u64 {
    fallback
}

#[cfg(not(test))]
#[inline]
unsafe fn wrmsr_raw(msr: u32, value: u64) {
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") (value & 0xffff_ffff) as u32,
            in("edx") (value >> 32) as u32,
            options(nostack, nomem, preserves_flags),
        );
    }
}

#[cfg(not(test))]
#[inline]
unsafe fn rdmsr_raw(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nostack, nomem, preserves_flags),
        );
    }
    ((high as u64) << 32) | low as u64
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use super::*;
    use crate::kernel::{cred::INIT_CRED, sched, task::TaskStruct};

    #[test]
    fn arch_prctl_constants_match_linux() {
        assert_eq!(ARCH_SET_GS, 0x1001);
        assert_eq!(ARCH_SET_FS, 0x1002);
        assert_eq!(ARCH_GET_FS, 0x1003);
        assert_eq!(ARCH_GET_GS, 0x1004);
    }

    #[test]
    fn syscall_m76_process_control_parity() {
        let previous = unsafe { sched::get_current() };
        let mut current = Box::new(unsafe { core::mem::zeroed::<TaskStruct>() });
        current.pid = 87;
        current.tgid = 87;
        current.cred = &raw const INIT_CRED;
        unsafe {
            sched::set_current(&mut *current as *mut TaskStruct);
            assert_eq!(sys_arch_prctl(ARCH_SET_FS, 0x7000), 0);
            let mut fs = 0u64;
            assert_eq!(sys_arch_prctl(ARCH_GET_FS, &mut fs as *mut u64 as u64), 0);
            assert_eq!(fs, 0x7000);
            assert_eq!(sys_arch_prctl(ARCH_SET_GS, 0x8000), 0);
            let mut gs = 0u64;
            assert_eq!(sys_arch_prctl(ARCH_GET_GS, &mut gs as *mut u64 as u64), 0);
            assert_eq!(gs, 0x8000);
            assert_eq!(sys_arch_prctl(ARCH_GET_FS, 0), EFAULT);
            assert_eq!(sys_arch_prctl(ARCH_GET_FS, uaccess::TASK_SIZE_MAX), EFAULT);
            assert_eq!(sys_arch_prctl(0, 0), EINVAL);
            sched::set_current(previous);
        }
    }
}
