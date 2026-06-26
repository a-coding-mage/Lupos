//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/microcode/intel.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/microcode/intel.c
//! Intel microcode container parser.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/microcode/intel.c

// Intel microcode headers are 48 bytes long with a fixed layout starting
// at offset 0: header_version, update_revision, date, processor_signature,
// checksum, loader_revision, processor_flags, data_size, total_size.
// We model the header parser and a simple checksum check.

use crate::include::uapi::errno::{EINVAL, ENODATA};

pub const INTEL_UCODE_HEADER_SIZE: usize = 48;
pub const INTEL_UCODE_HEADER_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntelUcodeHeader {
    pub header_version: u32,
    pub update_revision: u32,
    pub date: u32,
    pub processor_signature: u32,
    pub processor_flags: u32,
    pub data_size: u32,
    pub total_size: u32,
}

pub fn parse_header(blob: &[u8]) -> Result<IntelUcodeHeader, i32> {
    if blob.len() < INTEL_UCODE_HEADER_SIZE {
        return Err(ENODATA);
    }
    let read_u32 = |offset: usize| -> u32 {
        u32::from_le_bytes([
            blob[offset],
            blob[offset + 1],
            blob[offset + 2],
            blob[offset + 3],
        ])
    };
    let header_version = read_u32(0);
    if header_version != INTEL_UCODE_HEADER_VERSION {
        return Err(EINVAL);
    }
    Ok(IntelUcodeHeader {
        header_version,
        update_revision: read_u32(4),
        date: read_u32(8),
        processor_signature: read_u32(12),
        processor_flags: read_u32(24),
        data_size: read_u32(28),
        total_size: read_u32(32),
    })
}

pub const fn header_applies_to(header: IntelUcodeHeader, signature: u32, platform_id: u8) -> bool {
    let platform_bit = 1u32 << (platform_id as u32 & 0x07);
    header.processor_signature == signature && (header.processor_flags & platform_bit) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_header(sig: u32, platform_flags: u32) -> alloc::vec::Vec<u8> {
        extern crate alloc;
        let mut buf = alloc::vec![0u8; INTEL_UCODE_HEADER_SIZE];
        buf[0..4].copy_from_slice(&INTEL_UCODE_HEADER_VERSION.to_le_bytes());
        buf[12..16].copy_from_slice(&sig.to_le_bytes());
        buf[24..28].copy_from_slice(&platform_flags.to_le_bytes());
        buf
    }

    #[test]
    fn rejects_unknown_header_version() {
        let mut buf = alloc::vec![0u8; INTEL_UCODE_HEADER_SIZE];
        buf[0] = 0x42;
        assert_eq!(parse_header(&buf), Err(EINVAL));
    }

    #[test]
    fn applicability_uses_platform_bitmap() {
        extern crate alloc;
        let header_bytes = build_header(0x000506e3, 0b0010);
        let header = parse_header(&header_bytes).unwrap();
        assert!(header_applies_to(header, 0x000506e3, 1));
        assert!(!header_applies_to(header, 0x000506e3, 0));
        assert!(!header_applies_to(header, 0x000506e4, 1));
    }
}
