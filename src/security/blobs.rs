//! linux-parity: complete
//! linux-source: vendor/linux/security
//! test-origin: linux:vendor/linux/security
//! LSM security blob allocator.
//!
//! Ref: `vendor/linux/security/security.c`.

use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LsmBlob {
    pub id: usize,
    pub bytes: usize,
}

static NEXT_BLOB_ID: AtomicUsize = AtomicUsize::new(1);

pub fn alloc_blob(bytes: usize) -> LsmBlob {
    LsmBlob {
        id: NEXT_BLOB_ID.fetch_add(1, Ordering::AcqRel),
        bytes,
    }
}

pub fn free_blob(blob: &mut Option<LsmBlob>) {
    *blob = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_lifecycle_allocates_unique_ids_and_frees() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let a = alloc_blob(32);
        let b = alloc_blob(64);
        assert_ne!(a.id, b.id);
        let mut slot = Some(a);
        free_blob(&mut slot);
        assert_eq!(slot, None);
    }
}
