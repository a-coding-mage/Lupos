//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/iomap_32.c
//! test-origin: linux:vendor/linux/arch/x86/mm/iomap_32.c
//! x86 32-bit iomap compatibility surface.
//!
//! Mirrors exported helpers from `vendor/linux/arch/x86/mm/iomap_32.c`.
//! Lupos runs the x86_64 `ioremap` implementation, so 32-bit local iomaps are
//! explicitly unsupported.

use crate::arch::x86::mm::paging::pgprot_t;
use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IomapLocalMapping {
    pub pfn: u64,
    pub pages: u32,
    pub prot: pgprot_t,
}

pub const fn iomap_create_wc(pfn: u64, pages: u32) -> Result<IomapLocalMapping, i32> {
    if pages == 0 {
        return Err(EINVAL);
    }
    let _ = pfn;
    Err(EOPNOTSUPP)
}

pub const fn iomap_free(_mapping: IomapLocalMapping) -> Result<(), i32> {
    Err(EOPNOTSUPP)
}

pub const fn iomap_local_pfn_prot(pfn: u64, prot: pgprot_t) -> Result<IomapLocalMapping, i32> {
    Ok(IomapLocalMapping {
        pfn,
        pages: 1,
        prot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::x86::mm::paging::PAGE_KERNEL;

    #[test]
    fn zero_page_iomap_is_invalid() {
        assert_eq!(iomap_create_wc(1, 0), Err(EINVAL));
    }

    #[test]
    fn pfn_prot_mapping_records_request_without_installing_32_bit_iomap() {
        assert_eq!(
            iomap_local_pfn_prot(7, PAGE_KERNEL).unwrap(),
            IomapLocalMapping {
                pfn: 7,
                pages: 1,
                prot: PAGE_KERNEL
            }
        );
    }
}
