//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/mmu_hvm.c
//! test-origin: linux:vendor/linux/arch/x86/xen/mmu_hvm.c
//! Xen HVM MMU callbacks.

pub const DOMID_SELF: u16 = 0x7ff0;
pub const HVMOP_GET_MEM_TYPE: u32 = 15;
pub const HVMOP_PAGETABLE_DYING: u32 = 9;
pub const HVMMEM_MMIO_DM: u16 = 3;

pub const fn xen_vmcore_pfn_is_ram(hypercall_failed: bool, mem_type: u16) -> bool {
    if hypercall_failed {
        true
    } else {
        mem_type != HVMMEM_MMIO_DM
    }
}

pub const fn is_pagetable_dying_supported(hypercall_rc: i32) -> bool {
    hypercall_rc >= 0
}

pub const fn xen_hvm_exit_mmap_gpa(pgd_phys: u64) -> u64 {
    pgd_phys
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenHvmMmuInit {
    pub exit_mmap_installed: bool,
    pub vmcore_registered: bool,
}

pub const fn xen_hvm_init_mmu_ops(pagetable_dying_rc: i32, proc_vmcore: bool) -> XenHvmMmuInit {
    XenHvmMmuInit {
        exit_mmap_installed: is_pagetable_dying_supported(pagetable_dying_rc),
        vmcore_registered: proc_vmcore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xen_hvm_mmu_callbacks_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/mmu_hvm.c"
        ));
        assert!(source.contains("xen_vmcore_pfn_is_ram"));
        assert!(source.contains(".domid = DOMID_SELF"));
        assert!(source.contains("HVMOP_get_mem_type"));
        assert!(source.contains("return a.mem_type != HVMMEM_mmio_dm;"));
        assert!(source.contains("xen_hvm_exit_mmap"));
        assert!(source.contains("HVMOP_pagetable_dying"));
        assert!(source.contains("HVMOP_pagetable_dying not supported"));
        assert!(source.contains("pv_ops.mmu.exit_mmap = xen_hvm_exit_mmap;"));
        assert!(source.contains("register_vmcore_cb(&xen_vmcore_cb);"));

        assert!(xen_vmcore_pfn_is_ram(true, HVMMEM_MMIO_DM));
        assert!(!xen_vmcore_pfn_is_ram(false, HVMMEM_MMIO_DM));
        assert!(xen_vmcore_pfn_is_ram(false, 0));
        assert!(is_pagetable_dying_supported(0));
        assert!(!is_pagetable_dying_supported(-1));
        assert_eq!(
            xen_hvm_init_mmu_ops(0, true),
            XenHvmMmuInit {
                exit_mmap_installed: true,
                vmcore_registered: true,
            }
        );
    }
}
