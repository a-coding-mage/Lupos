//! linux-parity: complete
//! linux-source: vendor/linux/fs/ceph/ceph_frag.c
//! test-origin: linux:vendor/linux/fs/ceph/ceph_frag.c
//! Ceph frag logical comparator.

pub const fn ceph_frag_bits(frag: u32) -> u32 {
    frag >> 24
}

pub const fn ceph_frag_value(frag: u32) -> u32 {
    frag & 0x00ff_ffff
}

pub const fn ceph_frag_make(bits: u32, value: u32) -> u32 {
    (bits << 24) | (value & (0x00ff_ffffu32 << (24 - bits)) & 0x00ff_ffff)
}

pub const fn ceph_frag_compare(a: u32, b: u32) -> i32 {
    let va = ceph_frag_value(a);
    let vb = ceph_frag_value(b);
    if va < vb {
        return -1;
    }
    if va > vb {
        return 1;
    }

    let ba = ceph_frag_bits(a);
    let bb = ceph_frag_bits(b);
    if ba < bb {
        -1
    } else if ba > bb {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_frag_compare_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ceph/ceph_frag.c"
        ));
        assert!(source.contains("#include <linux/ceph/types.h>"));
        assert!(source.contains("ceph_frag_value(a)"));
        assert!(source.contains("ceph_frag_bits(a)"));
        assert_eq!(
            ceph_frag_compare(ceph_frag_make(1, 0), ceph_frag_make(1, 0)),
            0
        );
        assert_eq!(
            ceph_frag_compare(ceph_frag_make(1, 0), ceph_frag_make(1, 0x800000)),
            -1
        );
        assert_eq!(
            ceph_frag_compare(ceph_frag_make(1, 0x800000), ceph_frag_make(1, 0)),
            1
        );
        assert_eq!(
            ceph_frag_compare(ceph_frag_make(1, 0), ceph_frag_make(2, 0)),
            -1
        );
    }
}
