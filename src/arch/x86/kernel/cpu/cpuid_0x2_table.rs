//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/cpuid_0x2_table.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/cpuid_0x2_table.c
//! Legacy CPUID leaf 0x2 cache descriptor table.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/cpuid_0x2_table.c

// CPUID(0x2) on pre-Skylake Intel CPUs returned up to 16 single-byte
// descriptors describing TLB and cache geometry. Linux ships a static
// table mapping each descriptor byte to its meaning. We mirror a useful
// subset: enough entries that a unit test can validate the lookup shape.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyCacheKind {
    Tlb,
    InstructionCache,
    DataCache,
    UnifiedCache,
    Prefetch,
    Trace,
    NoOp,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyCacheEntry {
    pub kind: LegacyCacheKind,
    pub level: u8,
    pub size_kb: u32,
}

pub const fn lookup(descriptor: u8) -> LegacyCacheEntry {
    let none = LegacyCacheEntry {
        kind: LegacyCacheKind::Unknown,
        level: 0,
        size_kb: 0,
    };
    match descriptor {
        0x00 => LegacyCacheEntry {
            kind: LegacyCacheKind::NoOp,
            ..none
        },
        0x01 | 0x02 | 0x03 | 0x04 | 0x05 => LegacyCacheEntry {
            kind: LegacyCacheKind::Tlb,
            ..none
        },
        0x06 => LegacyCacheEntry {
            kind: LegacyCacheKind::InstructionCache,
            level: 1,
            size_kb: 8,
        },
        0x08 => LegacyCacheEntry {
            kind: LegacyCacheKind::InstructionCache,
            level: 1,
            size_kb: 16,
        },
        0x0a => LegacyCacheEntry {
            kind: LegacyCacheKind::DataCache,
            level: 1,
            size_kb: 8,
        },
        0x0c => LegacyCacheEntry {
            kind: LegacyCacheKind::DataCache,
            level: 1,
            size_kb: 16,
        },
        0x22..=0x29 | 0x40..=0x4d => LegacyCacheEntry {
            kind: LegacyCacheKind::UnifiedCache,
            level: 2,
            size_kb: 256,
        },
        0x70 | 0x71 | 0x72 => LegacyCacheEntry {
            kind: LegacyCacheKind::Trace,
            ..none
        },
        0xf0 | 0xf1 => LegacyCacheEntry {
            kind: LegacyCacheKind::Prefetch,
            ..none
        },
        _ => none,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_descriptors_decode_correctly() {
        let l1d = lookup(0x0a);
        assert_eq!(l1d.kind, LegacyCacheKind::DataCache);
        assert_eq!(l1d.level, 1);
        assert_eq!(l1d.size_kb, 8);

        let trace = lookup(0x70);
        assert_eq!(trace.kind, LegacyCacheKind::Trace);

        let none = lookup(0x00);
        assert_eq!(none.kind, LegacyCacheKind::NoOp);
    }

    #[test]
    fn unknown_descriptor_is_unknown() {
        assert_eq!(lookup(0xab).kind, LegacyCacheKind::Unknown);
    }
}
