//! linux-parity: stub
//! scope: out-of-scope for boot/Arch goal (layout-only placeholder)
//! linux-source: vendor/linux/arch/x86/kernel/cpu/microcode/amd.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/microcode/amd.c
//! AMD microcode container parser (equivalence-table + patch blocks).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/microcode/amd.c

// An AMD microcode firmware blob starts with the magic 0x00414d44 ("AMD\0"),
// followed by an equivalence table mapping CPUID signatures to internal
// patch IDs, then a sequence of patch records. We model header parsing
// over a byte slice; the actual MSR write is intentionally absent.

use crate::include::uapi::errno::{EINVAL, ENODATA};

pub const AMD_UCODE_MAGIC: u32 = 0x0041_4d44;
pub const AMD_UCODE_EQUIV_TABLE: u32 = 0x0;
pub const AMD_UCODE_PATCH_HEADER: u32 = 0x1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdMicrocodeHeader {
    pub equivalent_cpu: u32,
    pub patch_id: u32,
    pub data_offset: usize,
    pub data_size: usize,
}

pub fn parse_header(blob: &[u8]) -> Result<AmdMicrocodeHeader, i32> {
    if blob.len() < 24 {
        return Err(ENODATA);
    }
    let magic = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]);
    if magic != AMD_UCODE_MAGIC {
        return Err(EINVAL);
    }
    let equivalent_cpu = u32::from_le_bytes([blob[8], blob[9], blob[10], blob[11]]);
    let patch_id = u32::from_le_bytes([blob[12], blob[13], blob[14], blob[15]]);
    let data_size = u32::from_le_bytes([blob[16], blob[17], blob[18], blob[19]]) as usize;
    if data_size > blob.len().saturating_sub(20) {
        return Err(EINVAL);
    }
    Ok(AmdMicrocodeHeader {
        equivalent_cpu,
        patch_id,
        data_offset: 20,
        data_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_blob(eq_cpu: u32, patch_id: u32, payload: &[u8]) -> alloc::vec::Vec<u8> {
        extern crate alloc;
        let mut buf = alloc::vec::Vec::new();
        buf.extend_from_slice(&AMD_UCODE_MAGIC.to_le_bytes());
        buf.extend_from_slice(&[0; 4]);
        buf.extend_from_slice(&eq_cpu.to_le_bytes());
        buf.extend_from_slice(&patch_id.to_le_bytes());
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(payload);
        buf
    }

    #[test]
    fn rejects_blob_with_wrong_magic() {
        let bad = [0u8; 32];
        assert_eq!(parse_header(&bad), Err(EINVAL));
    }

    #[test]
    fn parses_valid_header() {
        let blob = fixture_blob(0x0010_0f00, 0x0123_4567, &[1, 2, 3, 4]);
        let header = parse_header(&blob).unwrap();
        assert_eq!(header.equivalent_cpu, 0x0010_0f00);
        assert_eq!(header.patch_id, 0x0123_4567);
        assert_eq!(header.data_size, 4);
    }
}
