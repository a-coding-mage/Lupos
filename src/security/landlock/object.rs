//! linux-parity: complete
//! linux-source: vendor/linux/security/landlock/object.c
//! test-origin: linux:vendor/linux/security/landlock/object.c
//! Landlock object lifetime and release management.

use crate::include::uapi::errno::{ENOENT, ENOMEM};

pub struct LandlockObjectUnderops {
    pub release: fn(&mut LandlockObject),
}

pub struct LandlockObject {
    usage: usize,
    underops: &'static LandlockObjectUnderops,
    underobj: Option<usize>,
    released: bool,
}

impl LandlockObject {
    pub fn usage(&self) -> usize {
        self.usage
    }

    pub fn underobj(&self) -> Option<usize> {
        self.underobj
    }

    pub fn released(&self) -> bool {
        self.released
    }

    pub fn clear_underobj(&mut self) {
        self.underobj = None;
    }
}

pub fn landlock_create_object(
    underops: Option<&'static LandlockObjectUnderops>,
    underobj: Option<usize>,
) -> Result<LandlockObject, i32> {
    let Some(underops) = underops else {
        return Err(-ENOENT);
    };
    let Some(underobj) = underobj else {
        return Err(-ENOENT);
    };
    if underobj == usize::MAX {
        return Err(-ENOMEM);
    }
    Ok(LandlockObject {
        usage: 1,
        underops,
        underobj: Some(underobj),
        released: false,
    })
}

pub fn landlock_get_object(object: &mut LandlockObject) {
    if !object.released {
        object.usage += 1;
    }
}

pub fn landlock_put_object(object: Option<&mut LandlockObject>) {
    let Some(object) = object else {
        return;
    };
    if object.usage == 0 || object.released {
        return;
    }

    object.usage -= 1;
    if object.usage == 0 {
        (object.underops.release)(object);
        object.released = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    static RELEASES: AtomicUsize = AtomicUsize::new(0);

    fn release(object: &mut LandlockObject) {
        RELEASES.fetch_add(1, Ordering::AcqRel);
        object.clear_underobj();
    }

    static UNDEROPS: LandlockObjectUnderops = LandlockObjectUnderops { release };

    #[test]
    fn landlock_object_refcount_releases_underobj_at_zero() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        RELEASES.store(0, Ordering::Release);

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/landlock/object.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/landlock/object.h"
        ));
        assert!(source.contains("landlock_create_object"));
        assert!(source.contains("refcount_set(&new_object->usage, 1)"));
        assert!(source.contains("refcount_dec_and_lock(&object->usage, &object->lock)"));
        assert!(source.contains("object->underops->release(object)"));
        assert!(header.contains("landlock_get_object"));

        assert_eq!(landlock_create_object(None, Some(1)).err(), Some(-ENOENT));
        assert_eq!(
            landlock_create_object(Some(&UNDEROPS), None).err(),
            Some(-ENOENT)
        );

        let mut object = landlock_create_object(Some(&UNDEROPS), Some(0xfeed)).expect("object");
        assert_eq!(object.usage(), 1);
        assert_eq!(object.underobj(), Some(0xfeed));
        landlock_get_object(&mut object);
        assert_eq!(object.usage(), 2);
        landlock_put_object(Some(&mut object));
        assert_eq!(object.usage(), 1);
        assert_eq!(RELEASES.load(Ordering::Acquire), 0);
        landlock_put_object(Some(&mut object));
        assert_eq!(object.usage(), 0);
        assert!(object.released());
        assert_eq!(object.underobj(), None);
        assert_eq!(RELEASES.load(Ordering::Acquire), 1);
        landlock_put_object(Some(&mut object));
        assert_eq!(RELEASES.load(Ordering::Acquire), 1);
        landlock_put_object(None);
    }
}
