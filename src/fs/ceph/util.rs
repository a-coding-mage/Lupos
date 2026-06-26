//! linux-parity: complete
//! linux-source: vendor/linux/fs/ceph/util.c
//! test-origin: linux:vendor/linux/fs/ceph/util.c
//! Ceph layout validation and open-mode capability helpers.

use crate::include::uapi::fcntl::{O_ACCMODE, O_DIRECTORY, O_RDONLY, O_RDWR, O_WRONLY};

pub const CEPH_MIN_STRIPE_UNIT: u32 = 65_536;

pub const CEPH_FILE_MODE_PIN: u32 = 0;
pub const CEPH_FILE_MODE_RD: u32 = 1;
pub const CEPH_FILE_MODE_WR: u32 = 2;
pub const CEPH_FILE_MODE_RDWR: u32 = 3;
pub const CEPH_FILE_MODE_LAZY: u32 = 4;

pub const CEPH_CAP_PIN: u32 = 1;
pub const CEPH_CAP_AUTH_SHARED: u32 = 1 << 2;
pub const CEPH_CAP_AUTH_EXCL: u32 = 2 << 2;
pub const CEPH_CAP_XATTR_SHARED: u32 = 1 << 6;
pub const CEPH_CAP_XATTR_EXCL: u32 = 2 << 6;
pub const CEPH_CAP_FILE_SHARED: u32 = 1 << 8;
pub const CEPH_CAP_FILE_EXCL: u32 = 2 << 8;
pub const CEPH_CAP_FILE_CACHE: u32 = 4 << 8;
pub const CEPH_CAP_FILE_RD: u32 = 8 << 8;
pub const CEPH_CAP_FILE_WR: u32 = 16 << 8;
pub const CEPH_CAP_FILE_BUFFER: u32 = 32 << 8;
pub const CEPH_CAP_FILE_LAZYIO: u32 = 128 << 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CephFileLayout {
    pub stripe_unit: u32,
    pub stripe_count: u32,
    pub object_size: u32,
    pub pool_id: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CephFileLayoutLegacy {
    pub stripe_unit: u32,
    pub stripe_count: u32,
    pub object_size: u32,
    pub pg_pool: u32,
}

pub const fn ceph_file_layout_is_valid(layout: CephFileLayout) -> bool {
    let su = layout.stripe_unit;
    let sc = layout.stripe_count;
    let os = layout.object_size;
    if su == 0 || (su & (CEPH_MIN_STRIPE_UNIT - 1)) != 0 {
        return false;
    }
    if os == 0 || (os & (CEPH_MIN_STRIPE_UNIT - 1)) != 0 {
        return false;
    }
    if os < su || os % su != 0 {
        return false;
    }
    sc != 0
}

pub const fn ceph_file_layout_from_legacy(legacy: CephFileLayoutLegacy) -> CephFileLayout {
    let mut pool_id = legacy.pg_pool as i64;
    if pool_id == 0
        && legacy.stripe_unit == 0
        && legacy.stripe_count == 0
        && legacy.object_size == 0
    {
        pool_id = -1;
    }
    CephFileLayout {
        stripe_unit: legacy.stripe_unit,
        stripe_count: legacy.stripe_count,
        object_size: legacy.object_size,
        pool_id,
    }
}

pub const fn ceph_file_layout_to_legacy(layout: CephFileLayout) -> CephFileLayoutLegacy {
    CephFileLayoutLegacy {
        stripe_unit: layout.stripe_unit,
        stripe_count: layout.stripe_count,
        object_size: layout.object_size,
        pg_pool: if layout.pool_id >= 0 {
            layout.pool_id as u32
        } else {
            0
        },
    }
}

pub const fn ceph_flags_to_mode(flags: u32) -> u32 {
    if flags & O_DIRECTORY == O_DIRECTORY {
        return CEPH_FILE_MODE_PIN;
    }
    match flags & O_ACCMODE {
        O_WRONLY => CEPH_FILE_MODE_WR,
        O_RDONLY => CEPH_FILE_MODE_RD,
        O_RDWR | O_ACCMODE => CEPH_FILE_MODE_RDWR,
        _ => CEPH_FILE_MODE_PIN,
    }
}

pub const fn ceph_caps_for_mode(mode: u32) -> u32 {
    let mut caps = CEPH_CAP_PIN;
    if mode & CEPH_FILE_MODE_RD != 0 {
        caps |= CEPH_CAP_FILE_SHARED | CEPH_CAP_FILE_RD | CEPH_CAP_FILE_CACHE;
    }
    if mode & CEPH_FILE_MODE_WR != 0 {
        caps |= CEPH_CAP_FILE_EXCL
            | CEPH_CAP_FILE_WR
            | CEPH_CAP_FILE_BUFFER
            | CEPH_CAP_AUTH_SHARED
            | CEPH_CAP_AUTH_EXCL
            | CEPH_CAP_XATTR_SHARED
            | CEPH_CAP_XATTR_EXCL;
    }
    if mode & CEPH_FILE_MODE_LAZY != 0 {
        caps |= CEPH_CAP_FILE_LAZYIO;
    }
    caps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_util_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ceph/util.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/ceph/types.h>"));
        assert!(source.contains("int ceph_file_layout_is_valid"));
        assert!(source.contains("if (!su || (su & (CEPH_MIN_STRIPE_UNIT-1)))"));
        assert!(source.contains("if (!os || (os & (CEPH_MIN_STRIPE_UNIT-1)))"));
        assert!(source.contains("if (os < su || os % su)"));
        assert!(source.contains("if (!sc)"));
        assert!(source.contains("ceph_file_layout_from_legacy"));
        assert!(source.contains("fl->pool_id = le32_to_cpu(legacy->fl_pg_pool);"));
        assert!(source.contains("fl->pool_id = -1;"));
        assert!(source.contains("ceph_file_layout_to_legacy"));
        assert!(source.contains("if (fl->pool_id >= 0)"));
        assert!(source.contains("int ceph_flags_to_mode"));
        assert!(source.contains("return CEPH_FILE_MODE_PIN;"));
        assert!(source.contains("case O_WRONLY:"));
        assert!(source.contains("case O_RDONLY:"));
        assert!(source.contains("case O_RDWR:"));
        assert!(source.contains("case O_ACCMODE:"));
        assert!(source.contains("int ceph_caps_for_mode"));
        assert!(source.contains("int caps = CEPH_CAP_PIN;"));
        assert!(source.contains("CEPH_CAP_FILE_SHARED"));
        assert!(source.contains("CEPH_CAP_AUTH_EXCL"));
        assert!(source.contains("CEPH_CAP_FILE_LAZYIO"));

        let valid = CephFileLayout {
            stripe_unit: CEPH_MIN_STRIPE_UNIT,
            stripe_count: 2,
            object_size: CEPH_MIN_STRIPE_UNIT * 4,
            pool_id: 9,
        };
        assert!(ceph_file_layout_is_valid(valid));
        assert!(!ceph_file_layout_is_valid(CephFileLayout {
            stripe_unit: 1,
            stripe_count: 1,
            object_size: CEPH_MIN_STRIPE_UNIT,
            pool_id: 0,
        }));
        assert_eq!(
            ceph_file_layout_from_legacy(CephFileLayoutLegacy {
                stripe_unit: 0,
                stripe_count: 0,
                object_size: 0,
                pg_pool: 0,
            })
            .pool_id,
            -1
        );
        assert_eq!(ceph_file_layout_to_legacy(valid).pg_pool, 9);
        assert_eq!(ceph_flags_to_mode(O_RDONLY), CEPH_FILE_MODE_RD);
        assert_eq!(ceph_flags_to_mode(O_WRONLY), CEPH_FILE_MODE_WR);
        assert_eq!(ceph_flags_to_mode(O_RDWR), CEPH_FILE_MODE_RDWR);
        assert_eq!(ceph_flags_to_mode(O_DIRECTORY), CEPH_FILE_MODE_PIN);
        assert_eq!(
            ceph_caps_for_mode(CEPH_FILE_MODE_RD),
            CEPH_CAP_PIN | CEPH_CAP_FILE_SHARED | CEPH_CAP_FILE_RD | CEPH_CAP_FILE_CACHE
        );
        assert!(ceph_caps_for_mode(CEPH_FILE_MODE_WR) & CEPH_CAP_AUTH_EXCL != 0);
    }
}
