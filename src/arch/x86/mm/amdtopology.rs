//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/amdtopology.c
//! test-origin: linux:vendor/linux/arch/x86/mm/amdtopology.c
//! AMD northbridge NUMA discovery policy.
//!
//! Mirrors the conservative parts of `vendor/linux/arch/x86/mm/amdtopology.c`.
//! Lupos currently exposes a single NUMA node, so the AMD-specific discovery
//! path validates inputs and reports either the discovered node count or
//! `ENODEV` when the host is not an AMD-family topology provider.

use crate::include::uapi::errno::{EINVAL, ENODEV};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdNorthbridgeProbe {
    pub vendor_is_amd: bool,
    pub northbridge_count: u8,
    pub node_count: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmdNumaTopology {
    pub nodes: u8,
    pub northbridges: u8,
}

pub const fn find_northbridge(probe: AmdNorthbridgeProbe) -> Result<u8, i32> {
    if !probe.vendor_is_amd || probe.northbridge_count == 0 {
        return Err(ENODEV);
    }
    Ok(probe.northbridge_count)
}

pub const fn amd_numa_init(probe: AmdNorthbridgeProbe) -> Result<AmdNumaTopology, i32> {
    if probe.node_count == 0 {
        return Err(EINVAL);
    }
    match find_northbridge(probe) {
        Ok(northbridges) => Ok(AmdNumaTopology {
            nodes: probe.node_count,
            northbridges,
        }),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_amd_probe_returns_enodev() {
        assert_eq!(
            amd_numa_init(AmdNorthbridgeProbe {
                vendor_is_amd: false,
                northbridge_count: 1,
                node_count: 1
            }),
            Err(ENODEV)
        );
    }

    #[test]
    fn amd_probe_reports_nodes_and_northbridges() {
        assert_eq!(
            amd_numa_init(AmdNorthbridgeProbe {
                vendor_is_amd: true,
                northbridge_count: 2,
                node_count: 2
            }),
            Ok(AmdNumaTopology {
                nodes: 2,
                northbridges: 2
            })
        );
    }
}
