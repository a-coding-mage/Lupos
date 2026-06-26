//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/tdx/debug.c
//! test-origin: linux:vendor/linux/arch/x86/coco/tdx/debug.c
//! TDX attribute debug decoders.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/tdx/debug.c

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedBits {
    pub names: [&'static str; 16],
    pub len: usize,
    pub unknown: u64,
}

impl DecodedBits {
    const fn empty() -> Self {
        Self {
            names: [""; 16],
            len: 0,
            unknown: 0,
        }
    }

    fn push(&mut self, name: &'static str) {
        self.names[self.len] = name;
        self.len += 1;
    }
}

pub const TDX_TD_ATTR_DEBUG_BIT: u8 = 0;
pub const TDX_TD_ATTR_HGS_PLUS_PROF_BIT: u8 = 4;
pub const TDX_TD_ATTR_PERF_PROF_BIT: u8 = 5;
pub const TDX_TD_ATTR_PMT_PROF_BIT: u8 = 6;
pub const TDX_TD_ATTR_ICSSD_BIT: u8 = 16;
pub const TDX_TD_ATTR_LASS_BIT: u8 = 27;
pub const TDX_TD_ATTR_SEPT_VE_DISABLE_BIT: u8 = 28;
pub const TDX_TD_ATTR_MIGRATABLE_BIT: u8 = 29;
pub const TDX_TD_ATTR_PKS_BIT: u8 = 30;
pub const TDX_TD_ATTR_KL_BIT: u8 = 31;
pub const TDX_TD_ATTR_TPA_BIT: u8 = 62;
pub const TDX_TD_ATTR_PERFMON_BIT: u8 = 63;

pub const TD_CTLS_PENDING_VE_DISABLE_BIT: u8 = 0;
pub const TD_CTLS_ENUM_TOPOLOGY_BIT: u8 = 1;
pub const TD_CTLS_VIRT_CPUID2_BIT: u8 = 2;
pub const TD_CTLS_REDUCE_VE_BIT: u8 = 3;
pub const TD_CTLS_LOCK_BIT: u8 = 63;

const ATTR_NAMES: &[(u8, &str)] = &[
    (TDX_TD_ATTR_DEBUG_BIT, "DEBUG"),
    (TDX_TD_ATTR_HGS_PLUS_PROF_BIT, "HGS_PLUS_PROF"),
    (TDX_TD_ATTR_PERF_PROF_BIT, "PERF_PROF"),
    (TDX_TD_ATTR_PMT_PROF_BIT, "PMT_PROF"),
    (TDX_TD_ATTR_ICSSD_BIT, "ICSSD"),
    (TDX_TD_ATTR_LASS_BIT, "LASS"),
    (TDX_TD_ATTR_SEPT_VE_DISABLE_BIT, "SEPT_VE_DISABLE"),
    (TDX_TD_ATTR_MIGRATABLE_BIT, "MIGRATABLE"),
    (TDX_TD_ATTR_PKS_BIT, "PKS"),
    (TDX_TD_ATTR_KL_BIT, "KL"),
    (TDX_TD_ATTR_TPA_BIT, "TPA"),
    (TDX_TD_ATTR_PERFMON_BIT, "PERFMON"),
];

const TD_CTLS_NAMES: &[(u8, &str)] = &[
    (TD_CTLS_PENDING_VE_DISABLE_BIT, "PENDING_VE_DISABLE"),
    (TD_CTLS_ENUM_TOPOLOGY_BIT, "ENUM_TOPOLOGY"),
    (TD_CTLS_VIRT_CPUID2_BIT, "VIRT_CPUID2"),
    (TD_CTLS_REDUCE_VE_BIT, "REDUCE_VE"),
    (TD_CTLS_LOCK_BIT, "LOCK"),
];

pub fn tdx_dump_attributes(td_attr: u64) -> DecodedBits {
    decode_bits(td_attr, ATTR_NAMES)
}

pub fn tdx_dump_td_ctls(td_ctls: u64) -> DecodedBits {
    decode_bits(td_ctls, TD_CTLS_NAMES)
}

fn decode_bits(mut value: u64, table: &[(u8, &'static str)]) -> DecodedBits {
    let mut decoded = DecodedBits::empty();
    for (bit, name) in table.iter().copied() {
        let mask = 1u64 << bit;
        if value & mask != 0 {
            decoded.push(name);
            value &= !mask;
        }
    }
    decoded.unknown = value;
    decoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_decoder_names_known_bits_and_preserves_unknown() {
        let decoded = tdx_dump_attributes((1 << TDX_TD_ATTR_DEBUG_BIT) | (1 << 12));
        assert_eq!(decoded.len, 1);
        assert_eq!(decoded.names[0], "DEBUG");
        assert_eq!(decoded.unknown, 1 << 12);
    }

    #[test]
    fn td_ctls_decoder_tracks_lock_high_bit() {
        let decoded = tdx_dump_td_ctls(1u64 << TD_CTLS_LOCK_BIT);
        assert_eq!(decoded.names[0], "LOCK");
        assert_eq!(decoded.unknown, 0);
    }
}
