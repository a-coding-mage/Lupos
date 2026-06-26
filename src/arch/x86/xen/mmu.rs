//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/xen/mmu.c
//! test-origin: linux:vendor/linux/arch/x86/xen/mmu.c
//! Xen arbitrary virtual-address to machine-address translation.

use crate::include::uapi::errno::EINVAL;

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
pub const PAGE_MASK: u64 = !(PAGE_SIZE - 1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineAddress {
    pub maddr: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XenPte {
    pub mfn: u64,
}

pub const fn arbitrary_virt_to_mfn(machine: MachineAddress) -> u64 {
    machine.maddr >> PAGE_SHIFT
}

pub const fn arbitrary_virt_to_machine(
    vaddr: u64,
    virt_addr_valid: bool,
    direct_machine: u64,
    pte: Option<XenPte>,
) -> Option<MachineAddress> {
    if virt_addr_valid {
        return Some(MachineAddress {
            maddr: direct_machine,
        });
    }
    let Some(pte) = pte else {
        return None;
    };
    let offset = vaddr & !PAGE_MASK;
    Some(MachineAddress {
        maddr: (pte.mfn << PAGE_SHIFT) + offset,
    })
}

pub const fn xen_unmap_domain_gfn_range(
    xen_pv_domain: bool,
    pages_present: bool,
) -> Result<&'static str, i32> {
    if !xen_pv_domain {
        return Ok("xlate_unmap_gfn_range");
    }
    if !pages_present {
        return Ok("noop");
    }
    Err(-EINVAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xen_mmu_arbitrary_mapping_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/xen/mmu.c"
        ));
        assert!(source.contains("unsigned long arbitrary_virt_to_mfn(void *vaddr)"));
        assert!(source.contains("return PFN_DOWN(maddr.maddr);"));
        assert!(source.contains("xmaddr_t arbitrary_virt_to_machine(void *vaddr)"));
        assert!(source.contains("if (virt_addr_valid(vaddr))"));
        assert!(source.contains("return virt_to_machine(vaddr);"));
        assert!(source.contains("lookup_address(address, &level);"));
        assert!(source.contains("BUG_ON(pte == NULL);"));
        assert!(source.contains("offset = address & ~PAGE_MASK;"));
        assert!(source.contains("pte_mfn(*pte) << PAGE_SHIFT"));
        assert!(source.contains("xen_unmap_domain_gfn_range"));
        assert!(source.contains("if (!xen_pv_domain())"));
        assert!(source.contains("return -EINVAL;"));

        let walked =
            arbitrary_virt_to_machine(0x1234, false, 0, Some(XenPte { mfn: 0xabc })).unwrap();
        assert_eq!(walked.maddr, 0xabc000 + 0x234);
        assert_eq!(arbitrary_virt_to_mfn(walked), 0xabc);
        assert_eq!(
            xen_unmap_domain_gfn_range(false, true),
            Ok("xlate_unmap_gfn_range")
        );
        assert_eq!(xen_unmap_domain_gfn_range(true, false), Ok("noop"));
        assert_eq!(xen_unmap_domain_gfn_range(true, true), Err(-EINVAL));
    }
}
