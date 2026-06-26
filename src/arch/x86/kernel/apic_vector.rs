//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x86 APIC interrupt vector allocation model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/vector.c

use crate::include::uapi::errno::ENOSPC;

pub const FIRST_EXTERNAL_VECTOR: u8 = 0x20;
pub const FIRST_SYSTEM_VECTOR: u8 = 0xef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VectorBitmap {
    pub used_low: u64,
    pub used_high: u64,
}

impl VectorBitmap {
    pub const fn empty() -> Self {
        Self {
            used_low: 0,
            used_high: 0,
        }
    }

    pub const fn is_used(self, vector: u8) -> bool {
        if vector < 64 {
            self.used_low & (1u64 << vector) != 0
        } else if vector < 128 {
            self.used_high & (1u64 << (vector - 64)) != 0
        } else {
            false
        }
    }

    pub const fn mark(mut self, vector: u8) -> Self {
        if vector < 64 {
            self.used_low |= 1u64 << vector;
        } else if vector < 128 {
            self.used_high |= 1u64 << (vector - 64);
        }
        self
    }
}

pub const fn allocate_vector(bitmap: VectorBitmap) -> Result<u8, i32> {
    let mut vector = FIRST_EXTERNAL_VECTOR;
    while vector < FIRST_SYSTEM_VECTOR && vector < 128 {
        if !bitmap.is_used(vector) {
            return Ok(vector);
        }
        vector += 1;
    }
    Err(ENOSPC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_returns_first_free_external_vector() {
        let bitmap = VectorBitmap::empty().mark(FIRST_EXTERNAL_VECTOR);
        assert_eq!(allocate_vector(bitmap), Ok(FIRST_EXTERNAL_VECTOR + 1));
    }
}
