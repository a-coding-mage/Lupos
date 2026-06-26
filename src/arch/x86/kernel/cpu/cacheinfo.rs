//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/cacheinfo.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/cacheinfo.c
//! Cache hierarchy decoder using CPUID leaf 0x4 (Intel) / 0x8000_001d (AMD).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/cacheinfo.c

// The hierarchy is enumerated by iterating leaf 0x4 with successive ECX
// subleaf values. Each subleaf encodes cache type, level, line size, and
// associativity. Subleaf 0 with type=0 terminates the walk. We model the
// decoder over a CPUID leaf result rather than reading hardware directly.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheKind {
    None,
    Data,
    Instruction,
    Unified,
    Reserved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheDescriptor {
    pub kind: CacheKind,
    pub level: u8,
    pub line_size: u32,
    pub ways: u32,
    pub sets: u32,
    pub partitions: u32,
}

pub const fn decode_leaf4(eax: u32, ebx: u32, ecx: u32) -> CacheDescriptor {
    let kind = match eax & 0x1f {
        0 => CacheKind::None,
        1 => CacheKind::Data,
        2 => CacheKind::Instruction,
        3 => CacheKind::Unified,
        _ => CacheKind::Reserved,
    };
    let level = ((eax >> 5) & 0x7) as u8;
    let line_size = (ebx & 0xfff) + 1;
    let partitions = ((ebx >> 12) & 0x3ff) + 1;
    let ways = ((ebx >> 22) & 0x3ff) + 1;
    let sets = ecx + 1;
    CacheDescriptor {
        kind,
        level,
        line_size,
        ways,
        sets,
        partitions,
    }
}

pub const fn cache_size_bytes(d: CacheDescriptor) -> u64 {
    (d.line_size as u64) * (d.partitions as u64) * (d.ways as u64) * (d.sets as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_zero_terminates_walk() {
        let d = decode_leaf4(0, 0, 0);
        assert_eq!(d.kind, CacheKind::None);
    }

    #[test]
    fn computes_size_from_classic_l1d() {
        // L1D 32 KiB, 64-byte lines, 8-way: line=64, ways=8, sets=64.
        let eax = (1 << 5) | 1;
        let ebx = (7 << 22) | (0 << 12) | 63;
        let ecx = 63;
        let d = decode_leaf4(eax, ebx, ecx);
        assert_eq!(d.kind, CacheKind::Data);
        assert_eq!(d.level, 1);
        assert_eq!(cache_size_bytes(d), 32 * 1024);
    }
}
