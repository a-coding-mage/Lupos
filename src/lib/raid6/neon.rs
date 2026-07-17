//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid6/neon.c
//! test-origin: linux:vendor/linux/lib/raid6/neon.c
//! ARM NEON RAID6 syndrome wrapper metadata.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Raid6NeonCall {
    pub name: &'static str,
    pub unroll: u8,
    pub wraps_scoped_ksimd: bool,
    pub priority: i32,
}

pub const RAID6_NEON_CALLS: &[Raid6NeonCall] = &[
    Raid6NeonCall {
        name: "neonx1",
        unroll: 1,
        wraps_scoped_ksimd: true,
        priority: 0,
    },
    Raid6NeonCall {
        name: "neonx2",
        unroll: 2,
        wraps_scoped_ksimd: true,
        priority: 0,
    },
    Raid6NeonCall {
        name: "neonx4",
        unroll: 4,
        wraps_scoped_ksimd: true,
        priority: 0,
    },
    Raid6NeonCall {
        name: "neonx8",
        unroll: 8,
        wraps_scoped_ksimd: true,
        priority: 0,
    },
];

pub const fn raid6_have_neon(cpu_has_neon: bool) -> i32 {
    cpu_has_neon as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raid6_neon_wrappers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid6/neon.c"
        ));
        assert!(source.contains("#include <linux/raid/pq.h>"));
        assert!(source.contains("#include <asm/neon.h>"));
        assert!(source.contains("#define RAID6_NEON_WRAPPER(_n)"));
        assert!(source.contains("kernel_neon_begin();"));
        assert!(source.contains("kernel_neon_end();"));
        assert!(source.contains("raid6_have_neon"));
        assert!(source.contains("return cpu_has_neon();"));
        assert!(source.contains("RAID6_NEON_WRAPPER(1);"));
        assert!(source.contains("RAID6_NEON_WRAPPER(2);"));
        assert!(source.contains("RAID6_NEON_WRAPPER(4);"));
        assert!(source.contains("RAID6_NEON_WRAPPER(8);"));

        assert_eq!(RAID6_NEON_CALLS.len(), 4);
        assert_eq!(RAID6_NEON_CALLS[3].name, "neonx8");
        assert_eq!(raid6_have_neon(true), 1);
        assert_eq!(raid6_have_neon(false), 0);
    }
}
