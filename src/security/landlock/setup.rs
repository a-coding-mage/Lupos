//! linux-parity: complete
//! linux-source: vendor/linux/security/landlock/setup.c
//! test-origin: linux:vendor/linux/security/landlock/setup.c
//! Landlock LSM setup and errata computation.

use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use crate::security::hooks::LSM_ID_LANDLOCK;

pub const LANDLOCK_NAME: &str = "landlock";
pub const LANDLOCK_ABI_VERSION: i32 = 9;

static LANDLOCK_INITIALIZED: AtomicBool = AtomicBool::new(false);
static LANDLOCK_ERRATA: AtomicI32 = AtomicI32::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LandlockErratum {
    pub abi: i32,
    pub number: u8,
}

pub const LANDLOCK_ERRATA_INIT: [LandlockErratum; 3] = [
    LandlockErratum { abi: 1, number: 3 },
    LandlockErratum { abi: 4, number: 1 },
    LandlockErratum { abi: 6, number: 2 },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LandlockBlobSizes {
    pub lbs_cred: usize,
    pub lbs_file: usize,
    pub lbs_inode: usize,
    pub lbs_superblock: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LandlockLsmId {
    pub name: &'static str,
    pub id: u64,
}

pub const LANDLOCK_LSMID: LandlockLsmId = LandlockLsmId {
    name: LANDLOCK_NAME,
    id: LSM_ID_LANDLOCK,
};

pub fn landlock_blob_sizes() -> LandlockBlobSizes {
    LandlockBlobSizes {
        lbs_cred: core::mem::size_of::<super::cred::LandlockCredSecurity>(),
        lbs_file: core::mem::size_of::<usize>(),
        lbs_inode: core::mem::size_of::<usize>(),
        lbs_superblock: core::mem::size_of::<usize>(),
    }
}

pub fn compute_errata() -> i32 {
    let mut errata = 0i32;
    for entry in LANDLOCK_ERRATA_INIT {
        if entry.abi <= LANDLOCK_ABI_VERSION {
            errata |= 1 << (entry.number - 1);
        }
    }
    LANDLOCK_ERRATA.store(errata, Ordering::Release);
    errata
}

pub fn landlock_init() -> i32 {
    compute_errata();
    super::cred::landlock_add_cred_hooks();
    super::register_hooks();
    LANDLOCK_INITIALIZED.store(true, Ordering::Release);
    crate::kernel::printk::log_info!(LANDLOCK_NAME, "Up and running.");
    0
}

pub fn landlock_initialized() -> bool {
    LANDLOCK_INITIALIZED.load(Ordering::Acquire)
}

pub fn landlock_errata() -> i32 {
    LANDLOCK_ERRATA.load(Ordering::Acquire)
}

#[cfg(test)]
pub fn reset_for_test() {
    LANDLOCK_INITIALIZED.store(false, Ordering::Release);
    LANDLOCK_ERRATA.store(0, Ordering::Release);
    super::cred::reset_for_test();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landlock_setup_computes_errata_and_registers_lsm_hooks() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/landlock/setup.c"
        ));
        let errata = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/landlock/errata.h"
        ));
        assert!(source.contains("bool landlock_initialized"));
        assert!(source.contains("const struct lsm_id landlock_lsmid"));
        assert!(source.contains("compute_errata();"));
        assert!(source.contains("landlock_add_cred_hooks();"));
        assert!(source.contains("DEFINE_LSM(LANDLOCK_NAME)"));
        assert!(errata.contains("LANDLOCK_ERRATA_ABI 1"));
        assert!(errata.contains("LANDLOCK_ERRATA_ABI 4"));
        assert!(errata.contains("LANDLOCK_ERRATA_ABI 6"));

        assert_eq!(LANDLOCK_LSMID.id, LSM_ID_LANDLOCK);
        assert_eq!(LANDLOCK_ABI_VERSION, 9);
        assert_eq!(compute_errata(), 0b111);
        assert_eq!(landlock_errata(), 0b111);
        assert!(landlock_blob_sizes().lbs_cred > 0);

        assert_eq!(landlock_init(), 0);
        assert!(landlock_initialized());
        assert!(super::super::cred::cred_hooks_registered());

        let mut ids = [0u64; 4];
        let count = crate::security::lsm_list::lsm_active_ids(&mut ids);
        assert_eq!(count, 1);
        assert_eq!(ids[0], LSM_ID_LANDLOCK);
    }
}
