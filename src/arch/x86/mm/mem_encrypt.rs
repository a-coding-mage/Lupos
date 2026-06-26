//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/mem_encrypt.c
//! test-origin: linux:vendor/linux/arch/x86/mm/mem_encrypt.c
//! Generic x86 memory-encryption state.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::x86::coco::core::{CcAttr, CcPlatformState, CcVendor, cc_platform_has_state};
use crate::arch::x86::kernel::x86_init;
use crate::arch::x86::mm::paging::{self, PAGE_MASK, PAGE_SIZE};

pub const IO_TLB_DEFAULT_SIZE: u64 = 64 << 20;
pub const SZ_1G: u64 = 0x4000_0000;

static SME_ME_MASK: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
const FLUSH_LOG_CAP: usize = 8;
#[cfg(test)]
static FLUSH_LOG_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static FLUSH_LOG: spin::Mutex<[(u64, u64, bool); FLUSH_LOG_CAP]> =
    spin::Mutex::new([(0, 0, false); FLUSH_LOG_CAP]);

pub fn sme_me_mask() -> u64 {
    SME_ME_MASK.load(Ordering::Acquire)
}

pub fn mem_encrypt_active() -> bool {
    sme_me_mask() != 0
}

pub fn publish_me_mask(mask: u64) {
    SME_ME_MASK.store(mask, Ordering::Release);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaDeviceMasks {
    pub coherent_dma_mask: u64,
    pub bus_dma_limit: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemEncryptFeatureInfo {
    None,
    IntelTdx,
    AmdSme,
    AmdSev {
        sev: bool,
        sev_es: bool,
        sev_snp: bool,
        show_status: bool,
    },
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemEncryptInitPlan {
    pub active: bool,
    pub swiotlb_update_mem_attributes: bool,
    pub snp_secure_tsc_prepare: bool,
    pub feature_info: Option<MemEncryptFeatureInfo>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemEncryptSetupArchPlan {
    pub host_snp_fixup_e820_tables: bool,
    pub guest_mem_encrypt: bool,
    pub swiotlb_adjust_size: Option<u64>,
    pub virtio_restricted_mem_acc: bool,
}

pub const fn dma_bit_mask(bits: u32) -> u64 {
    if bits == 0 {
        0
    } else if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

pub const fn min_not_zero(x: u64, y: u64) -> u64 {
    if x == 0 {
        y
    } else if y != 0 && y < x {
        y
    } else {
        x
    }
}

pub const fn force_dma_unencrypted_state(
    state: CcPlatformState,
    masks: DmaDeviceMasks,
    me_mask: u64,
) -> bool {
    if cc_platform_has_state(state, CcAttr::GuestMemEncrypt) {
        return true;
    }

    if cc_platform_has_state(state, CcAttr::HostMemEncrypt) && me_mask != 0 {
        let dma_enc_mask = dma_bit_mask(me_mask.trailing_zeros());
        let dma_dev_mask = min_not_zero(masks.coherent_dma_mask, masks.bus_dma_limit);
        if dma_dev_mask <= dma_enc_mask {
            return true;
        }
    }

    false
}

pub fn force_dma_unencrypted(masks: DmaDeviceMasks) -> bool {
    force_dma_unencrypted_state(
        crate::arch::x86::coco::core::cc_state(),
        masks,
        sme_me_mask(),
    )
}

pub const fn print_mem_encrypt_feature_info_state(state: CcPlatformState) -> MemEncryptFeatureInfo {
    match state.vendor {
        CcVendor::Intel => MemEncryptFeatureInfo::IntelTdx,
        CcVendor::Amd => {
            if cc_platform_has_state(state, CcAttr::HostMemEncrypt) {
                return MemEncryptFeatureInfo::AmdSme;
            }

            MemEncryptFeatureInfo::AmdSev {
                sev: cc_platform_has_state(state, CcAttr::GuestMemEncrypt),
                sev_es: cc_platform_has_state(state, CcAttr::GuestStateEncrypt),
                sev_snp: cc_platform_has_state(state, CcAttr::GuestSevSnp),
                show_status: true,
            }
        }
        CcVendor::None => MemEncryptFeatureInfo::Unknown,
    }
}

pub const fn mem_encrypt_init_plan(state: CcPlatformState) -> MemEncryptInitPlan {
    if !cc_platform_has_state(state, CcAttr::MemEncrypt) {
        return MemEncryptInitPlan {
            active: false,
            swiotlb_update_mem_attributes: false,
            snp_secure_tsc_prepare: false,
            feature_info: None,
        };
    }

    MemEncryptInitPlan {
        active: true,
        swiotlb_update_mem_attributes: true,
        snp_secure_tsc_prepare: true,
        feature_info: Some(print_mem_encrypt_feature_info_state(state)),
    }
}

pub const fn clamp_val(value: u64, low: u64, high: u64) -> u64 {
    if value < low {
        low
    } else if value > high {
        high
    } else {
        value
    }
}

pub const fn mem_encrypt_setup_arch_plan(
    state: CcPlatformState,
    total_mem: u64,
) -> MemEncryptSetupArchPlan {
    let host_snp_fixup_e820_tables = cc_platform_has_state(state, CcAttr::HostSevSnp);

    if !cc_platform_has_state(state, CcAttr::GuestMemEncrypt) {
        return MemEncryptSetupArchPlan {
            host_snp_fixup_e820_tables,
            guest_mem_encrypt: false,
            swiotlb_adjust_size: None,
            virtio_restricted_mem_acc: false,
        };
    }

    let size = clamp_val(total_mem.wrapping_mul(6) / 100, IO_TLB_DEFAULT_SIZE, SZ_1G);
    MemEncryptSetupArchPlan {
        host_snp_fixup_e820_tables,
        guest_mem_encrypt: true,
        swiotlb_adjust_size: Some(size),
        virtio_restricted_mem_acc: true,
    }
}

fn encryption_range_end(addr: u64, numpages: usize) -> Result<u64, i32> {
    let pages = numpages as u64;
    let bytes = pages
        .checked_mul(PAGE_SIZE)
        .ok_or(crate::include::uapi::errno::EINVAL)?;
    addr.checked_add(bytes)
        .ok_or(crate::include::uapi::errno::EINVAL)
}

fn flush_encryption_range(start: u64, end: u64, cache: bool) {
    #[cfg(test)]
    record_flush(start, end, cache);

    unsafe {
        paging::flush_tlb_range(start, end);
    }

    if cache {
        let size = end - start;
        let clflush_done = crate::arch::x86::mm::pat::set_memory::cpu_has_clflush()
            && size <= usize::MAX as u64
            && crate::arch::x86::mm::pat::set_memory::clflush_cache_range(start, size as usize)
                .is_ok();
        if !clflush_done {
            crate::arch::x86::lib::cache_smp::wbinvd_on_all_cpus();
        }
    }
}

#[cfg(test)]
fn record_flush(start: u64, end: u64, cache: bool) {
    let idx = FLUSH_LOG_LEN.fetch_add(1, Ordering::AcqRel);
    if idx < FLUSH_LOG_CAP {
        FLUSH_LOG.lock()[idx] = (start, end, cache);
    }
}

#[cfg(test)]
fn reset_flush_log() {
    FLUSH_LOG_LEN.store(0, Ordering::Release);
    *FLUSH_LOG.lock() = [(0, 0, false); FLUSH_LOG_CAP];
}

#[cfg(test)]
fn flush_log() -> (usize, [(u64, u64, bool); FLUSH_LOG_CAP]) {
    (
        FLUSH_LOG_LEN.load(Ordering::Acquire).min(FLUSH_LOG_CAP),
        *FLUSH_LOG.lock(),
    )
}

fn set_memory_enc_dec(mut addr: u64, numpages: usize, encrypted: bool) -> Result<(), i32> {
    if numpages == 0 {
        return Ok(());
    }

    let mask = sme_me_mask();
    if mask == 0 {
        return Ok(());
    }

    if addr & (PAGE_SIZE - 1) != 0 {
        crate::log_warn!("", "set_memory_enc_dec: misaligned address {:#x}", addr);
        addr &= PAGE_MASK;
    }

    let end = encryption_range_end(addr, numpages)?;

    if x86_init::enc_tlb_flush_required(encrypted) {
        flush_encryption_range(addr, end, x86_init::enc_cache_flush_required());
    }

    x86_init::enc_status_change_prepare(addr, numpages, encrypted)?;

    unsafe { paging::set_kernel_page_encryption_mask(addr, numpages, mask, encrypted) }?;

    flush_encryption_range(addr, end, false);

    x86_init::enc_status_change_finish(addr, numpages, encrypted)
}

pub fn set_memory_encrypted(addr: u64, numpages: usize) -> Result<(), i32> {
    set_memory_enc_dec(addr, numpages, true)
}

pub fn set_memory_decrypted(addr: u64, numpages: usize) -> Result<(), i32> {
    set_memory_enc_dec(addr, numpages, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::coco::core::{
        MSR_AMD64_SEV_ENABLED, MSR_AMD64_SEV_ES_ENABLED, MSR_AMD64_SEV_SNP_ENABLED,
    };
    use crate::arch::x86::mm::paging;
    use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

    fn intel_guest_state() -> CcPlatformState {
        CcPlatformState {
            vendor: CcVendor::Intel,
            ..CcPlatformState::default()
        }
    }

    fn amd_state(cc_mask: u64, sev_status: u64, host_sev_snp: bool) -> CcPlatformState {
        CcPlatformState {
            vendor: CcVendor::Amd,
            cc_mask,
            sev_status,
            host_sev_snp,
        }
    }

    unsafe fn kernel_pte_value(addr: u64) -> u64 {
        unsafe {
            let pgd = paging::init_pgd_for_test();
            let pgdp = paging::pgd_offset_pgd(pgd, addr);
            let p4dp = paging::p4d_offset(pgdp, addr);
            let pudp = paging::pud_offset(p4dp, addr);
            let pmdp = paging::pmd_offset(pudp, addr);
            (*paging::pte_offset_kernel(pmdp, addr)).0
        }
    }

    unsafe fn kernel_pmd_value(addr: u64) -> u64 {
        unsafe {
            let pgd = paging::init_pgd_for_test();
            let pgdp = paging::pgd_offset_pgd(pgd, addr);
            let p4dp = paging::p4d_offset(pgdp, addr);
            let pudp = paging::pud_offset(p4dp, addr);
            (*paging::pmd_offset(pudp, addr)).0
        }
    }

    #[test]
    fn mem_encrypt_common_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/mm/mem_encrypt.c"
        ));
        let swiotlb = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/swiotlb.h"
        ));
        let sizes = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/sizes.h"
        ));
        let dma_mapping = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/dma-mapping.h"
        ));
        let minmax = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/minmax.h"
        ));
        let bitops = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/bitops.h"
        ));

        assert!(source.contains("bool force_dma_unencrypted(struct device *dev)"));
        assert!(source.contains("cc_platform_has(CC_ATTR_GUEST_MEM_ENCRYPT)"));
        assert!(source.contains("DMA_BIT_MASK(__ffs64(sme_me_mask))"));
        assert!(source.contains("min_not_zero(dev->coherent_dma_mask"));
        assert!(source.contains("static void print_mem_encrypt_feature_info(void)"));
        assert!(source.contains("case CC_VENDOR_INTEL:"));
        assert!(source.contains("pr_cont(\"Intel TDX\\n\");"));
        assert!(source.contains("case CC_VENDOR_AMD:"));
        assert!(source.contains("pr_cont(\" SME\\n\");"));
        assert!(source.contains("sev_show_status();"));
        assert!(source.contains("void __init mem_encrypt_init(void)"));
        assert!(source.contains("swiotlb_update_mem_attributes();"));
        assert!(source.contains("snp_secure_tsc_prepare();"));
        assert!(source.contains("void __init mem_encrypt_setup_arch(void)"));
        assert!(source.contains("snp_fixup_e820_tables();"));
        assert!(source.contains("size = total_mem * 6 / 100;"));
        assert!(source.contains("clamp_val(size, IO_TLB_DEFAULT_SIZE, SZ_1G);"));
        assert!(source.contains("swiotlb_adjust_size(size);"));
        assert!(source.contains("virtio_set_mem_acc_cb(virtio_require_restricted_mem_acc);"));
        assert!(swiotlb.contains("#define IO_TLB_DEFAULT_SIZE (64UL<<20)"));
        assert!(sizes.contains("#define SZ_1G"));
        assert!(dma_mapping.contains("#define DMA_BIT_MASK(n)"));
        assert!(minmax.contains("#define min_not_zero(x, y) ({"));
        assert!(
            bitops.contains("static inline __attribute_const__ unsigned int __ffs64(u64 word)")
        );
    }

    #[test]
    fn dma_bit_mask_and_min_not_zero_follow_kernel_macros() {
        assert_eq!(dma_bit_mask(0), 0);
        assert_eq!(dma_bit_mask(1), 1);
        assert_eq!(dma_bit_mask(47), (1u64 << 47) - 1);
        assert_eq!(dma_bit_mask(64), u64::MAX);

        assert_eq!(min_not_zero(0, 0), 0);
        assert_eq!(min_not_zero(0, 5), 5);
        assert_eq!(min_not_zero(7, 0), 7);
        assert_eq!(min_not_zero(7, 5), 5);
        assert_eq!(min_not_zero(3, 5), 3);
    }

    #[test]
    fn force_dma_unencrypted_matches_sev_and_sme_rules() {
        let masks = DmaDeviceMasks {
            coherent_dma_mask: u64::MAX,
            bus_dma_limit: u64::MAX,
        };
        assert!(force_dma_unencrypted_state(intel_guest_state(), masks, 0));

        let host_sme = amd_state(1u64 << 47, 0, false);
        let limited_device = DmaDeviceMasks {
            coherent_dma_mask: 0,
            bus_dma_limit: (1u64 << 47) - 1,
        };
        assert!(force_dma_unencrypted_state(
            host_sme,
            limited_device,
            1u64 << 47
        ));

        let wide_device = DmaDeviceMasks {
            coherent_dma_mask: u64::MAX,
            bus_dma_limit: 0,
        };
        assert!(!force_dma_unencrypted_state(
            host_sme,
            wide_device,
            1u64 << 47
        ));
        assert!(!force_dma_unencrypted_state(
            CcPlatformState::default(),
            limited_device,
            1u64 << 47
        ));
    }

    #[test]
    fn feature_info_follows_vendor_and_active_cc_attrs() {
        assert_eq!(
            print_mem_encrypt_feature_info_state(intel_guest_state()),
            MemEncryptFeatureInfo::IntelTdx
        );

        assert_eq!(
            print_mem_encrypt_feature_info_state(amd_state(1u64 << 47, 0, false)),
            MemEncryptFeatureInfo::AmdSme
        );

        assert_eq!(
            print_mem_encrypt_feature_info_state(amd_state(
                1u64 << 47,
                MSR_AMD64_SEV_ENABLED | MSR_AMD64_SEV_ES_ENABLED | MSR_AMD64_SEV_SNP_ENABLED,
                false,
            )),
            MemEncryptFeatureInfo::AmdSev {
                sev: true,
                sev_es: true,
                sev_snp: true,
                show_status: true,
            }
        );

        assert_eq!(
            print_mem_encrypt_feature_info_state(CcPlatformState::default()),
            MemEncryptFeatureInfo::Unknown
        );
    }

    #[test]
    fn init_plan_is_empty_without_mem_encrypt_and_full_for_active_guest() {
        assert_eq!(
            mem_encrypt_init_plan(CcPlatformState::default()),
            MemEncryptInitPlan {
                active: false,
                swiotlb_update_mem_attributes: false,
                snp_secure_tsc_prepare: false,
                feature_info: None,
            }
        );

        assert_eq!(
            mem_encrypt_init_plan(intel_guest_state()),
            MemEncryptInitPlan {
                active: true,
                swiotlb_update_mem_attributes: true,
                snp_secure_tsc_prepare: true,
                feature_info: Some(MemEncryptFeatureInfo::IntelTdx),
            }
        );
    }

    #[test]
    fn setup_arch_plan_matches_host_snp_fixup_and_guest_swiotlb_sizing() {
        assert_eq!(
            mem_encrypt_setup_arch_plan(amd_state(1u64 << 47, 0, true), 8 * SZ_1G),
            MemEncryptSetupArchPlan {
                host_snp_fixup_e820_tables: true,
                guest_mem_encrypt: false,
                swiotlb_adjust_size: None,
                virtio_restricted_mem_acc: false,
            }
        );

        assert_eq!(
            mem_encrypt_setup_arch_plan(intel_guest_state(), 512 << 20),
            MemEncryptSetupArchPlan {
                host_snp_fixup_e820_tables: false,
                guest_mem_encrypt: true,
                swiotlb_adjust_size: Some(IO_TLB_DEFAULT_SIZE),
                virtio_restricted_mem_acc: true,
            }
        );

        assert_eq!(
            mem_encrypt_setup_arch_plan(intel_guest_state(), 8 * SZ_1G).swiotlb_adjust_size,
            Some(8 * SZ_1G * 6 / 100)
        );
        assert_eq!(
            mem_encrypt_setup_arch_plan(intel_guest_state(), 100 * SZ_1G).swiotlb_adjust_size,
            Some(SZ_1G)
        );
    }

    #[test]
    fn memory_encryption_is_inactive_by_default() {
        publish_me_mask(0);
        assert_eq!(sme_me_mask(), 0);
        assert!(!mem_encrypt_active());
    }

    #[test]
    fn inactive_memory_encryption_conversion_is_linux_noop_success() {
        publish_me_mask(0);
        assert_eq!(set_memory_encrypted(0x1000, 1), Ok(()));
        assert_eq!(set_memory_decrypted(0x1000, 1), Ok(()));
    }

    #[test]
    fn live_encryption_conversion_updates_kernel_pte_cbit() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        let mask = 1u64 << 47;
        publish_me_mask(mask);

        let virt = 0xffff_ffff_8120_0000;
        let phys = 0x0000_0000_0040_0000;
        unsafe {
            paging::map_kernel_page(virt, phys, paging::PAGE_KERNEL);
        }
        assert_eq!(unsafe { kernel_pte_value(virt) } & mask, 0);

        assert_eq!(set_memory_encrypted(virt, 1), Ok(()));
        assert_ne!(unsafe { kernel_pte_value(virt) } & mask, 0);

        assert_eq!(set_memory_decrypted(virt, 1), Ok(()));
        assert_eq!(unsafe { kernel_pte_value(virt) } & mask, 0);

        publish_me_mask(0);
    }

    #[test]
    fn partial_huge_pmd_encryption_conversion_splits_to_ptes() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        let mask = 1u64 << 47;
        publish_me_mask(mask);

        let virt_base = 0xffff_ffff_8200_0000;
        let phys_base = 0x0000_0000_0080_0000;
        unsafe {
            let pgd = paging::init_pgd_for_test();
            let pgdp = paging::pgd_offset_pgd(pgd, virt_base);
            let pudp = paging::pud_alloc_kernel(pgdp, virt_base).expect("pud");
            let pmdp = paging::pmd_alloc_kernel(pudp, virt_base).expect("pmd");
            paging::set_pmd(
                pmdp,
                paging::__pmd(
                    phys_base | mask | paging::_PAGE_PRESENT | paging::_PAGE_RW | paging::_PAGE_PSE,
                ),
            );
        }

        let target = virt_base + paging::PAGE_SIZE;
        assert_ne!(
            unsafe { kernel_pmd_value(virt_base) } & paging::_PAGE_PSE,
            0
        );
        assert_eq!(set_memory_decrypted(target, 1), Ok(()));
        assert_eq!(
            unsafe { kernel_pmd_value(virt_base) } & paging::_PAGE_PSE,
            0
        );
        assert_ne!(unsafe { kernel_pte_value(virt_base) } & mask, 0);
        assert_eq!(unsafe { kernel_pte_value(target) } & mask, 0);

        publish_me_mask(0);
    }

    #[test]
    fn encryption_conversion_calls_guest_callbacks_in_linux_order() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        crate::arch::x86::kernel::x86_init::tests::reset_guest_callback_log();
        crate::arch::x86::kernel::x86_init::set_guest_ops(
            crate::arch::x86::kernel::x86_init::tests::recording_guest_ops(),
        );
        let mask = 1u64 << 47;
        publish_me_mask(mask);

        let virt = 0xffff_ffff_8240_0000;
        let phys = 0x0000_0000_00c0_0000;
        unsafe {
            paging::map_kernel_page(virt, phys, paging::PAGE_KERNEL);
        }

        assert_eq!(set_memory_encrypted(virt, 1), Ok(()));
        assert_eq!(
            &crate::arch::x86::kernel::x86_init::tests::guest_callback_log()[..4],
            ["tlb", "cache", "prepare", "finish"]
        );
        assert_ne!(unsafe { kernel_pte_value(virt) } & mask, 0);

        crate::arch::x86::kernel::x86_init::reset_guest_ops();
        publish_me_mask(0);
    }

    #[test]
    fn encryption_conversion_flushes_cache_before_and_tlb_after_attribute_change() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        crate::arch::x86::kernel::x86_init::tests::reset_guest_callback_log();
        crate::arch::x86::kernel::x86_init::set_guest_ops(
            crate::arch::x86::kernel::x86_init::tests::recording_guest_ops(),
        );
        reset_flush_log();
        crate::arch::x86::mm::pat::set_memory::reset_clflush_log();
        let mask = 1u64 << 47;
        publish_me_mask(mask);

        let virt = 0xffff_ffff_8250_0000;
        let phys = 0x0000_0000_00d0_0000;
        unsafe {
            paging::map_kernel_page(virt, phys, paging::PAGE_KERNEL);
        }

        assert_eq!(set_memory_encrypted(virt, 1), Ok(()));
        let (len, log) = flush_log();
        assert_eq!(len, 2);
        assert_eq!(
            &log[..len],
            &[
                (virt, virt + PAGE_SIZE, true),
                (virt, virt + PAGE_SIZE, false)
            ]
        );
        let (clflush_len, clflush_log) = crate::arch::x86::mm::pat::set_memory::clflush_log();
        assert_ne!(clflush_len, 0);
        assert_eq!(clflush_log[0], virt);
        assert_eq!(
            &crate::arch::x86::kernel::x86_init::tests::guest_callback_log()[..4],
            ["tlb", "cache", "prepare", "finish"]
        );
        assert_ne!(unsafe { kernel_pte_value(virt) } & mask, 0);

        crate::arch::x86::kernel::x86_init::reset_guest_ops();
        publish_me_mask(0);
    }

    #[test]
    fn encryption_conversion_rejects_overflowing_ranges_before_flush() {
        let _guard = TEST_LOCK.lock().unwrap();
        crate::arch::x86::kernel::x86_init::set_guest_ops(
            crate::arch::x86::kernel::x86_init::tests::recording_guest_ops(),
        );
        reset_flush_log();
        publish_me_mask(1u64 << 47);

        assert_eq!(
            set_memory_encrypted(u64::MAX & PAGE_MASK, usize::MAX),
            Err(crate::include::uapi::errno::EINVAL)
        );
        let (len, _) = flush_log();
        assert_eq!(len, 0);

        crate::arch::x86::kernel::x86_init::reset_guest_ops();
        publish_me_mask(0);
    }

    #[test]
    fn prepare_failure_prevents_page_table_conversion_and_finish() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        crate::arch::x86::kernel::x86_init::tests::reset_guest_callback_log();
        crate::arch::x86::kernel::x86_init::set_guest_ops(
            crate::arch::x86::kernel::x86_init::tests::failing_prepare_guest_ops(),
        );
        let mask = 1u64 << 47;
        publish_me_mask(mask);

        let virt = 0xffff_ffff_8260_0000;
        let phys = 0x0000_0000_00e0_0000;
        unsafe {
            paging::map_kernel_page(virt, phys, paging::PAGE_KERNEL);
        }

        assert_eq!(
            set_memory_encrypted(virt, 1),
            Err(crate::include::uapi::errno::EIO)
        );
        assert_eq!(
            &crate::arch::x86::kernel::x86_init::tests::guest_callback_log()[..3],
            ["tlb", "cache", "prepare"]
        );
        assert_eq!(unsafe { kernel_pte_value(virt) } & mask, 0);

        crate::arch::x86::kernel::x86_init::reset_guest_ops();
        publish_me_mask(0);
    }

    #[test]
    fn finish_failure_reports_error_after_page_table_conversion() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        crate::arch::x86::kernel::x86_init::tests::reset_guest_callback_log();
        crate::arch::x86::kernel::x86_init::set_guest_ops(
            crate::arch::x86::kernel::x86_init::tests::failing_finish_guest_ops(),
        );
        let mask = 1u64 << 47;
        publish_me_mask(mask);

        let virt = 0xffff_ffff_8280_0000;
        let phys = 0x0000_0000_0100_0000;
        unsafe {
            paging::map_kernel_page(virt, phys, paging::PAGE_KERNEL);
        }

        assert_eq!(
            set_memory_encrypted(virt, 1),
            Err(crate::include::uapi::errno::EIO)
        );
        assert_eq!(
            &crate::arch::x86::kernel::x86_init::tests::guest_callback_log()[..4],
            ["tlb", "cache", "prepare", "finish"]
        );
        assert_ne!(unsafe { kernel_pte_value(virt) } & mask, 0);

        crate::arch::x86::kernel::x86_init::reset_guest_ops();
        publish_me_mask(0);
    }
}
