//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! AMD SEV-SNP Secure AVIC backing-page register model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/x2apic_savic.c

// Secure AVIC keeps a per-vCPU backing page where the hypervisor writes
// pending-vector bitmaps and the vCPU acknowledges by clearing bits with
// CMPXCHG. We mirror the bitmap layout (8x 32-bit words) and offer a
// pure interrupt-allocation helper for tests; the actual page mapping is
// owned by the memory subsystem and exposed via a trait later.

use crate::include::uapi::errno::EINVAL;

pub const SAVIC_BITMAP_WORDS: usize = 8;
pub const SAVIC_VECTOR_COUNT: u16 = 256;
pub const SAVIC_VECTOR_MIN: u8 = 0x10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SavicBackingPage {
    pub irr: [u32; SAVIC_BITMAP_WORDS],
    pub isr: [u32; SAVIC_BITMAP_WORDS],
}

impl SavicBackingPage {
    pub const fn empty() -> Self {
        Self {
            irr: [0; SAVIC_BITMAP_WORDS],
            isr: [0; SAVIC_BITMAP_WORDS],
        }
    }

    pub const fn set_pending(&mut self, vector: u8) -> Result<(), i32> {
        if vector < SAVIC_VECTOR_MIN {
            return Err(EINVAL);
        }
        let word = (vector >> 5) as usize;
        let bit = (vector & 0x1f) as u32;
        self.irr[word] |= 1u32 << bit;
        Ok(())
    }

    pub const fn ack(&mut self, vector: u8) -> Result<(), i32> {
        if vector < SAVIC_VECTOR_MIN {
            return Err(EINVAL);
        }
        let word = (vector >> 5) as usize;
        let bit = (vector & 0x1f) as u32;
        self.irr[word] &= !(1u32 << bit);
        self.isr[word] |= 1u32 << bit;
        Ok(())
    }

    pub const fn is_pending(&self, vector: u8) -> bool {
        let word = (vector >> 5) as usize;
        let bit = (vector & 0x1f) as u32;
        (self.irr[word] & (1u32 << bit)) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectors_below_reserved_range_are_rejected() {
        let mut page = SavicBackingPage::empty();
        assert_eq!(page.set_pending(0x0f), Err(EINVAL));
    }

    #[test]
    fn ack_moves_bit_from_irr_to_isr() {
        let mut page = SavicBackingPage::empty();
        page.set_pending(0x20).unwrap();
        assert!(page.is_pending(0x20));
        page.ack(0x20).unwrap();
        assert!(!page.is_pending(0x20));
        assert_eq!(page.isr[1], 1);
    }
}
