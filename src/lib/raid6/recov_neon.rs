//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid6/recov_neon.c
//! test-origin: linux:vendor/linux/lib/raid6/recov_neon.c
//! ARM NEON RAID6 recovery call metadata.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Raid6RecovCalls {
    pub name: &'static str,
    pub priority: i32,
    pub has_data2: bool,
    pub has_datap: bool,
}

pub const RAID6_RECOV_NEON: Raid6RecovCalls = Raid6RecovCalls {
    name: "neon",
    priority: 10,
    has_data2: true,
    has_datap: true,
};

pub const fn raid6_has_neon(cpu_has_neon: bool) -> i32 {
    cpu_has_neon as i32
}

pub const fn raid6_2data_restores_slots(disks: usize) -> (usize, usize) {
    (disks - 2, disks - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raid6_recov_neon_matches_linux_recovery_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid6/recov_neon.c"
        ));
        assert!(source.contains("#include <linux/raid/pq.h>"));
        assert!(source.contains("#include <asm/simd.h>"));
        assert!(source.contains("static int raid6_has_neon(void)"));
        assert!(source.contains("return cpu_has_neon();"));
        assert!(source.contains("ptrs[faila] = raid6_get_zero_page();"));
        assert!(source.contains("ptrs[disks - 2] = dp;"));
        assert!(source.contains("raid6_call.gen_syndrome(disks, bytes, ptrs);"));
        assert!(source.contains("pbmul = raid6_vgfmul[raid6_gfexi[failb-faila]];"));
        assert!(source.contains("qmul  = raid6_vgfmul[raid6_gfinv[raid6_gfexp[faila] ^"));
        assert!(source.contains("__raid6_2data_recov_neon(bytes, p, q, dp, dq, pbmul, qmul);"));
        assert!(source.contains("__raid6_datap_recov_neon(bytes, p, q, dq, qmul);"));
        assert!(source.contains("const struct raid6_recov_calls raid6_recov_neon"));
        assert!(source.contains(".name\t\t= \"neon\""));
        assert!(source.contains(".priority\t= 10"));

        assert_eq!(raid6_has_neon(true), 1);
        assert_eq!(raid6_has_neon(false), 0);
        assert_eq!(raid6_2data_restores_slots(8), (6, 7));
        assert_eq!(RAID6_RECOV_NEON.priority, 10);
    }
}
