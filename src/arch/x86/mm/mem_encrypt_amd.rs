//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/mm/mem_encrypt_amd.c
//! test-origin: linux:vendor/linux/arch/x86/mm/mem_encrypt_amd.c
//! AMD SME/SEV memory encryption helpers.
//!
//! Mirrors C-bit mask derivation from `vendor/linux/arch/x86/mm/mem_encrypt_amd.c`.
//! The calculated mask is explicit data; live encrypted/decrypted mapping
//! conversion is delegated to the generic x86 page-attribute layer.

use crate::arch::x86::coco::core::{CcAttr, cc_platform_has};
use crate::arch::x86::kernel::x86_init::X86GuestOps;
use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdMemEncryptInfo {
    pub sme_supported: bool,
    pub sev_supported: bool,
    pub c_bit_position: u8,
    pub physical_address_reduction: u8,
}

pub const fn amd_me_mask(info: AmdMemEncryptInfo) -> Result<u64, i32> {
    if !info.sme_supported {
        return Err(EOPNOTSUPP);
    }
    if info.c_bit_position >= 52 {
        return Err(EINVAL);
    }
    Ok(1u64 << info.c_bit_position)
}

pub fn sme_enable(info: AmdMemEncryptInfo) -> Result<u64, i32> {
    let mask = amd_me_mask(info)?;
    super::mem_encrypt::publish_me_mask(mask);
    crate::arch::x86::kernel::x86_init::set_guest_ops(amd_guest_ops());
    Ok(mask)
}

pub fn amd_guest_ops() -> X86GuestOps {
    X86GuestOps {
        enc_status_change_prepare: amd_enc_status_change_prepare,
        enc_status_change_finish: amd_enc_status_change_finish,
        enc_tlb_flush_required: amd_enc_tlb_flush_required,
        enc_cache_flush_required: amd_enc_cache_flush_required,
        enc_kexec_begin: || {},
        enc_kexec_finish: || {},
    }
}

fn amd_enc_tlb_flush_required(_enc: bool) -> bool {
    true
}

fn amd_enc_cache_flush_required() -> bool {
    true
}

fn amd_enc_status_change_prepare(vaddr: u64, npages: usize, enc: bool) -> Result<(), i32> {
    if cc_platform_has(CcAttr::GuestSevSnp) && !enc {
        crate::arch::x86::coco::sev::core::snp_set_memory_shared(vaddr, npages)?;
    }
    Ok(())
}

fn amd_enc_status_change_finish(vaddr: u64, npages: usize, enc: bool) -> Result<(), i32> {
    if cc_platform_has(CcAttr::GuestSevSnp) && enc {
        crate::arch::x86::coco::sev::core::snp_set_memory_private(vaddr, npages)?;
    }
    if !cc_platform_has(CcAttr::HostMemEncrypt) {
        amd_enc_dec_hypercall(vaddr, npages, enc);
    }
    Ok(())
}

fn amd_enc_dec_hypercall(vaddr: u64, npages: usize, enc: bool) {
    let Some(bytes) = (npages as u64).checked_mul(crate::arch::x86::mm::paging::PAGE_SIZE) else {
        return;
    };
    let Some(end) = vaddr.checked_add(bytes) else {
        return;
    };

    let mut addr = vaddr;
    while addr < end {
        let Some(phys) = crate::arch::x86::mm::paging::virt_to_phys(addr) else {
            crate::log_warn!("", "enc_dec_hypercall: kpte lookup for vaddr {:#x}", addr);
            return;
        };
        crate::arch::x86::kernel::paravirt::notify_page_enc_status_changed(
            phys >> crate::arch::x86::mm::paging::PAGE_SHIFT,
            1,
            enc,
        );
        addr = addr.saturating_add(crate::arch::x86::mm::paging::PAGE_SIZE);
    }
}

pub const fn sev_enabled(info: AmdMemEncryptInfo) -> bool {
    info.sev_supported && info.sme_supported
}

/// Linux `mem_encrypt_free_decrypted_mem()`.
///
/// The linker brackets the region that may have been mapped decrypted for
/// SME/SEV early boot. If an encryption mask is active, Linux first flips the
/// pages back to encrypted and only then frees them as ordinary init pages.
pub fn mem_encrypt_free_decrypted_mem_range(start: *mut u8, end: *mut u8) -> usize {
    if start.is_null() || end <= start {
        return 0;
    }

    let bytes = (end as usize).saturating_sub(start as usize);
    let pages = bytes / crate::mm::frame::PAGE_SIZE;
    if pages == 0 {
        return 0;
    }

    if super::mem_encrypt::sme_me_mask() != 0
        && super::mem_encrypt::set_memory_encrypted(start as u64, pages).is_err()
    {
        crate::log_warn!("", "failed to free unused decrypted pages");
        return 0;
    }

    let stats =
        unsafe { crate::mm::page_alloc::free_kernel_image_pages("unused decrypted", start, end) };
    stats.pages
}

#[cfg(not(test))]
pub fn mem_encrypt_free_decrypted_mem() -> usize {
    unsafe extern "C" {
        static __start_bss_decrypted_unused: u8;
        static __end_bss_decrypted: u8;
    }

    mem_encrypt_free_decrypted_mem_range(
        &raw const __start_bss_decrypted_unused as *mut u8,
        &raw const __end_bss_decrypted as *mut u8,
    )
}

#[cfg(test)]
pub fn mem_encrypt_free_decrypted_mem() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_bit_mask_is_derived_from_position() {
        let info = AmdMemEncryptInfo {
            sme_supported: true,
            sev_supported: false,
            c_bit_position: 47,
            physical_address_reduction: 1,
        };
        assert_eq!(amd_me_mask(info), Ok(1u64 << 47));
    }

    #[test]
    fn unsupported_sme_fails_closed() {
        let info = AmdMemEncryptInfo {
            sme_supported: false,
            sev_supported: false,
            c_bit_position: 0,
            physical_address_reduction: 0,
        };
        assert_eq!(amd_me_mask(info), Err(EOPNOTSUPP));
    }

    #[test]
    fn sme_enable_installs_amd_guest_callbacks() {
        let info = AmdMemEncryptInfo {
            sme_supported: true,
            sev_supported: true,
            c_bit_position: 47,
            physical_address_reduction: 1,
        };
        crate::arch::x86::kernel::x86_init::reset_guest_ops();

        assert_eq!(sme_enable(info), Ok(1u64 << 47));
        assert!(crate::arch::x86::kernel::x86_init::enc_tlb_flush_required(
            false
        ));
        assert!(crate::arch::x86::kernel::x86_init::enc_cache_flush_required());

        crate::arch::x86::kernel::x86_init::reset_guest_ops();
        super::super::mem_encrypt::publish_me_mask(0);
    }

    #[test]
    fn amd_snp_callbacks_route_shared_before_private_after() {
        use crate::arch::x86::coco::core::{
            CcPlatformState, CcVendor, MSR_AMD64_SEV_ENABLED, MSR_AMD64_SEV_ES_ENABLED,
            MSR_AMD64_SEV_SNP_ENABLED, publish_cc_state,
        };

        publish_cc_state(CcPlatformState {
            vendor: CcVendor::Amd,
            cc_mask: 1 << 47,
            sev_status: MSR_AMD64_SEV_ENABLED
                | MSR_AMD64_SEV_ES_ENABLED
                | MSR_AMD64_SEV_SNP_ENABLED,
            host_sev_snp: false,
        });
        let ops = amd_guest_ops();

        assert_eq!(
            (ops.enc_status_change_prepare)(
                0x4000,
                crate::arch::x86::coco::sev::core::VMGEXIT_PSC_MAX_ENTRY + 1,
                false,
            ),
            Ok(())
        );
        assert_eq!((ops.enc_status_change_prepare)(0x4000, 2, false), Ok(()));
        assert_eq!(
            (ops.enc_status_change_finish)(
                0x4000,
                crate::arch::x86::coco::sev::core::VMGEXIT_PSC_MAX_ENTRY + 1,
                true,
            ),
            Ok(())
        );
        assert_eq!((ops.enc_status_change_finish)(0x4000, 2, true), Ok(()));

        publish_cc_state(CcPlatformState::default());
    }

    #[test]
    fn amd_guest_finish_notifies_paravirt_when_not_host_encrypted() {
        use crate::arch::x86::coco::core::{
            CcPlatformState, CcVendor, MSR_AMD64_SEV_ENABLED, publish_cc_state,
        };
        use crate::arch::x86::mm::paging;
        use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        crate::arch::x86::kernel::paravirt::tests::reset_notify_page_enc_log();
        crate::arch::x86::kernel::paravirt::set_mmu_ops(
            crate::arch::x86::kernel::paravirt::tests::recording_mmu_ops(),
        );
        publish_cc_state(CcPlatformState {
            vendor: CcVendor::Amd,
            cc_mask: 1 << 47,
            sev_status: MSR_AMD64_SEV_ENABLED,
            host_sev_snp: false,
        });

        let virt = 0xffff_ffff_82a0_0000;
        let phys = 0x0000_0000_0120_0000;
        unsafe {
            paging::map_kernel_page(virt, phys, paging::PAGE_KERNEL);
        }

        assert_eq!(
            (amd_guest_ops().enc_status_change_finish)(virt, 1, false),
            Ok(())
        );
        assert_eq!(
            crate::arch::x86::kernel::paravirt::tests::notify_page_enc_log(),
            [(phys >> paging::PAGE_SHIFT, 1, false)]
        );

        crate::arch::x86::kernel::paravirt::reset_mmu_ops();
        publish_cc_state(CcPlatformState::default());
    }

    #[test]
    fn unused_decrypted_memory_empty_range_is_noop() {
        super::super::mem_encrypt::publish_me_mask(0);
        assert_eq!(
            mem_encrypt_free_decrypted_mem_range(0x4000 as *mut u8, 0x4000 as *mut u8),
            0
        );
    }

    #[test]
    fn unused_decrypted_memory_does_not_free_when_reencrypt_range_is_unmapped() {
        super::super::mem_encrypt::publish_me_mask(1);
        assert_eq!(
            mem_encrypt_free_decrypted_mem_range(0x4000 as *mut u8, 0x5000 as *mut u8),
            0
        );
        super::super::mem_encrypt::publish_me_mask(0);
    }
}
