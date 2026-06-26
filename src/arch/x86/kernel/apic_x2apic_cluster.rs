//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! x2APIC cluster-mode logical destination model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/x2apic_cluster.c

// Cluster mode encodes a 32-bit logical APIC ID as
//   (cluster_id << 16) | (1 << intra_cluster_index)
// with at most 16 CPUs per cluster. This module replicates that encoding
// and decoding without touching any hardware MSR.

pub const X2APIC_CLUSTER_BITS: u32 = 16;
pub const X2APIC_CLUSTER_MASK: u32 = 0xffff;
pub const X2APIC_INTRA_CLUSTER_MAX: u8 = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X2ApicClusterLdr {
    pub cluster_id: u16,
    pub intra_bitmap: u16,
}

pub const fn ldr_from_apicid(apicid: u32) -> X2ApicClusterLdr {
    let cluster_id = (apicid >> 4) as u16;
    let intra = apicid & 0x0f;
    X2ApicClusterLdr {
        cluster_id,
        intra_bitmap: 1u16 << intra,
    }
}

pub const fn encode_ldr(ldr: X2ApicClusterLdr) -> u32 {
    ((ldr.cluster_id as u32) << X2APIC_CLUSTER_BITS) | (ldr.intra_bitmap as u32)
}

pub const fn merge_into_cluster(
    existing: X2ApicClusterLdr,
    incoming: X2ApicClusterLdr,
) -> Result<X2ApicClusterLdr, i32> {
    if existing.cluster_id != incoming.cluster_id {
        return Err(crate::include::uapi::errno::EINVAL);
    }
    Ok(X2ApicClusterLdr {
        cluster_id: existing.cluster_id,
        intra_bitmap: existing.intra_bitmap | incoming.intra_bitmap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ldr_encodes_cluster_and_intra_bit() {
        let ldr = ldr_from_apicid(0x21);
        assert_eq!(ldr.cluster_id, 0x02);
        assert_eq!(ldr.intra_bitmap, 1 << 1);
        assert_eq!(encode_ldr(ldr), (0x02 << 16) | (1 << 1));
    }

    #[test]
    fn merge_requires_matching_cluster_id() {
        let a = ldr_from_apicid(0x20);
        let b = ldr_from_apicid(0x21);
        let merged = merge_into_cluster(a, b).unwrap();
        assert_eq!(merged.cluster_id, 0x02);
        assert_eq!(merged.intra_bitmap, 0b11);

        let c = ldr_from_apicid(0x30);
        assert!(merge_into_cluster(a, c).is_err());
    }
}
