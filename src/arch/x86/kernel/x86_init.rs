//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/x86_init.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/x86_init.c
//! x86 platform guest callback defaults.
//!
//! Linux keeps these callbacks in `x86_platform.guest`.  They let SEV, SNP,
//! TDX, and paravirt guests participate in page-encryption transitions without
//! making the generic page-attribute code know about each guest type.

use spin::Mutex;

/// Rust-shaped equivalent of Linux `struct x86_guest`.
///
/// Source: `vendor/linux/arch/x86/include/asm/x86_init.h:161`.
#[derive(Clone, Copy)]
pub struct X86GuestOps {
    pub enc_status_change_prepare: fn(u64, usize, bool) -> Result<(), i32>,
    pub enc_status_change_finish: fn(u64, usize, bool) -> Result<(), i32>,
    pub enc_tlb_flush_required: fn(bool) -> bool,
    pub enc_cache_flush_required: fn() -> bool,
    pub enc_kexec_begin: fn(),
    pub enc_kexec_finish: fn(),
}

fn enc_status_change_prepare_noop(_vaddr: u64, _npages: usize, _enc: bool) -> Result<(), i32> {
    Ok(())
}

fn enc_status_change_finish_noop(_vaddr: u64, _npages: usize, _enc: bool) -> Result<(), i32> {
    Ok(())
}

fn enc_tlb_flush_required_noop(_enc: bool) -> bool {
    false
}

fn enc_cache_flush_required_noop() -> bool {
    false
}

fn enc_kexec_noop() {}

impl X86GuestOps {
    pub const fn linux_noop() -> Self {
        Self {
            enc_status_change_prepare: enc_status_change_prepare_noop,
            enc_status_change_finish: enc_status_change_finish_noop,
            enc_tlb_flush_required: enc_tlb_flush_required_noop,
            enc_cache_flush_required: enc_cache_flush_required_noop,
            enc_kexec_begin: enc_kexec_noop,
            enc_kexec_finish: enc_kexec_noop,
        }
    }
}

static X86_GUEST_OPS: Mutex<X86GuestOps> = Mutex::new(X86GuestOps::linux_noop());

pub fn guest_ops() -> X86GuestOps {
    *X86_GUEST_OPS.lock()
}

pub fn set_guest_ops(ops: X86GuestOps) {
    *X86_GUEST_OPS.lock() = ops;
}

pub fn reset_guest_ops() {
    set_guest_ops(X86GuestOps::linux_noop());
}

pub fn enc_status_change_prepare(vaddr: u64, npages: usize, enc: bool) -> Result<(), i32> {
    ((guest_ops()).enc_status_change_prepare)(vaddr, npages, enc)
}

pub fn enc_status_change_finish(vaddr: u64, npages: usize, enc: bool) -> Result<(), i32> {
    ((guest_ops()).enc_status_change_finish)(vaddr, npages, enc)
}

pub fn enc_tlb_flush_required(enc: bool) -> bool {
    ((guest_ops()).enc_tlb_flush_required)(enc)
}

pub fn enc_cache_flush_required() -> bool {
    ((guest_ops()).enc_cache_flush_required)()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::include::uapi::errno::EIO;
    use core::sync::atomic::{AtomicUsize, Ordering};

    const LOG_CAP: usize = 8;
    static LOG_LEN: AtomicUsize = AtomicUsize::new(0);
    static LOG: Mutex<[&'static str; LOG_CAP]> = Mutex::new([""; LOG_CAP]);

    fn push(label: &'static str) {
        let idx = LOG_LEN.fetch_add(1, Ordering::AcqRel);
        if idx < LOG_CAP {
            LOG.lock()[idx] = label;
        }
    }

    pub fn reset_guest_callback_log() {
        LOG_LEN.store(0, Ordering::Release);
        *LOG.lock() = [""; LOG_CAP];
    }

    pub fn guest_callback_log() -> [&'static str; LOG_CAP] {
        *LOG.lock()
    }

    fn record_prepare(_vaddr: u64, _npages: usize, _enc: bool) -> Result<(), i32> {
        push("prepare");
        Ok(())
    }

    fn record_finish(_vaddr: u64, _npages: usize, _enc: bool) -> Result<(), i32> {
        push("finish");
        Ok(())
    }

    fn fail_prepare(_vaddr: u64, _npages: usize, _enc: bool) -> Result<(), i32> {
        push("prepare");
        Err(EIO)
    }

    fn fail_finish(_vaddr: u64, _npages: usize, _enc: bool) -> Result<(), i32> {
        push("finish");
        Err(EIO)
    }

    fn record_tlb(_enc: bool) -> bool {
        push("tlb");
        true
    }

    fn record_cache() -> bool {
        push("cache");
        true
    }

    pub fn recording_guest_ops() -> X86GuestOps {
        X86GuestOps {
            enc_status_change_prepare: record_prepare,
            enc_status_change_finish: record_finish,
            enc_tlb_flush_required: record_tlb,
            enc_cache_flush_required: record_cache,
            enc_kexec_begin: enc_kexec_noop,
            enc_kexec_finish: enc_kexec_noop,
        }
    }

    pub fn failing_prepare_guest_ops() -> X86GuestOps {
        X86GuestOps {
            enc_status_change_prepare: fail_prepare,
            ..recording_guest_ops()
        }
    }

    pub fn failing_finish_guest_ops() -> X86GuestOps {
        X86GuestOps {
            enc_status_change_finish: fail_finish,
            ..recording_guest_ops()
        }
    }

    #[test]
    fn x86_guest_default_callbacks_match_linux_noops() {
        reset_guest_ops();
        assert_eq!(enc_status_change_prepare(0x1000, 1, true), Ok(()));
        assert_eq!(enc_status_change_finish(0x1000, 1, false), Ok(()));
        assert!(!enc_tlb_flush_required(true));
        assert!(!enc_cache_flush_required());
    }
}
