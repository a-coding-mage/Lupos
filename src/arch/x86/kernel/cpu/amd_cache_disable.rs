//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/amd_cache_disable.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/amd_cache_disable.c
//! AMD L3 cache subcache disable (per-CPU sysfs control).
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/amd_cache_disable.c

// Family 0x10+ AMD CPUs expose six 16-bit subcache disable indices through
// PCI northbridge configuration registers F2 and F3. Each entry stores a
// disabled L3 way and an "active" bit. We model the index/way validation
// and the encoded register layout without touching the PCI fabric.

use crate::include::uapi::errno::EINVAL;

pub const AMD_L3_SUBCACHE_INDEX_COUNT: u8 = 4;
pub const AMD_L3_DISABLE_VALID: u32 = 1 << 30;
pub const AMD_L3_DISABLE_LOCKED: u32 = 1 << 31;
pub const AMD_L3_INDEX_MASK: u32 = 0x0fff;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdL3Disable {
    pub index: u8,
    pub way_index: u16,
    pub locked: bool,
}

pub const fn disable_register(entry: AmdL3Disable) -> Result<u32, i32> {
    if entry.index >= AMD_L3_SUBCACHE_INDEX_COUNT {
        return Err(EINVAL);
    }
    if (entry.way_index as u32) & !AMD_L3_INDEX_MASK != 0 {
        return Err(EINVAL);
    }
    let mut value = AMD_L3_DISABLE_VALID | (entry.way_index as u32);
    if entry.locked {
        value |= AMD_L3_DISABLE_LOCKED;
    }
    Ok(value)
}

pub const fn parse_disable_register(value: u32) -> Option<AmdL3Disable> {
    if value & AMD_L3_DISABLE_VALID == 0 {
        return None;
    }
    Some(AmdL3Disable {
        index: 0,
        way_index: (value & AMD_L3_INDEX_MASK) as u16,
        locked: value & AMD_L3_DISABLE_LOCKED != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_index_or_way_returns_einval() {
        assert_eq!(
            disable_register(AmdL3Disable {
                index: 5,
                way_index: 0,
                locked: false
            }),
            Err(EINVAL)
        );
        assert_eq!(
            disable_register(AmdL3Disable {
                index: 0,
                way_index: 0x2000,
                locked: false
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn round_trips_a_locked_entry() {
        let entry = AmdL3Disable {
            index: 0,
            way_index: 0x5,
            locked: true,
        };
        let encoded = disable_register(entry).unwrap();
        assert!(encoded & AMD_L3_DISABLE_VALID != 0);
        assert!(encoded & AMD_L3_DISABLE_LOCKED != 0);
        let parsed = parse_disable_register(encoded).unwrap();
        assert_eq!(parsed.way_index, 0x5);
        assert!(parsed.locked);
    }
}
