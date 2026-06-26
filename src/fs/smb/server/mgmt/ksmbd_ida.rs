//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/server/mgmt/ksmbd_ida.c
//! test-origin: linux:vendor/linux/fs/smb/server/mgmt/ksmbd_ida.c
//! KSMBD ID allocator wrappers.

extern crate alloc;

use alloc::collections::BTreeSet;

use crate::include::uapi::errno::{EINVAL, ENOSPC};

pub const SMB2_TID_MIN: u32 = 1;
pub const SMB2_TID_MAX: u32 = 0xffff_fffe;
pub const SMB2_UID_MIN: u32 = 1;
pub const SMB2_UID_RESERVED: u32 = 0xfffe;
pub const ASYNC_MSG_ID_MIN: u32 = 1;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KsmbdIda {
    allocated: BTreeSet<u32>,
}

impl KsmbdIda {
    pub fn new() -> Self {
        Self {
            allocated: BTreeSet::new(),
        }
    }

    pub fn contains(&self, id: u32) -> bool {
        self.allocated.contains(&id)
    }

    fn alloc_range(&mut self, min: u32, max: u32) -> Result<u32, i32> {
        if min > max {
            return Err(EINVAL);
        }
        let mut id = min;
        loop {
            if !self.allocated.contains(&id) {
                self.allocated.insert(id);
                return Ok(id);
            }
            if id == max {
                return Err(ENOSPC);
            }
            id = id.saturating_add(1);
        }
    }

    fn alloc_min(&mut self, min: u32) -> Result<u32, i32> {
        self.alloc_range(min, u32::MAX)
    }

    fn alloc(&mut self) -> Result<u32, i32> {
        self.alloc_min(0)
    }

    pub fn free(&mut self, id: u32) {
        self.allocated.remove(&id);
    }
}

pub fn ksmbd_acquire_smb2_tid(ida: &mut KsmbdIda) -> Result<u32, i32> {
    ida.alloc_range(SMB2_TID_MIN, SMB2_TID_MAX)
}

pub fn ksmbd_acquire_smb2_uid(ida: &mut KsmbdIda) -> Result<u32, i32> {
    let mut id = ida.alloc_min(SMB2_UID_MIN)?;
    if id == SMB2_UID_RESERVED {
        id = ida.alloc_min(SMB2_UID_MIN)?;
    }
    Ok(id)
}

pub fn ksmbd_acquire_async_msg_id(ida: &mut KsmbdIda) -> Result<u32, i32> {
    ida.alloc_min(ASYNC_MSG_ID_MIN)
}

pub fn ksmbd_acquire_id(ida: &mut KsmbdIda) -> Result<u32, i32> {
    ida.alloc()
}

pub fn ksmbd_release_id(ida: &mut KsmbdIda, id: u32) {
    ida.free(id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ksmbd_ida_wrappers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/server/mgmt/ksmbd_ida.c"
        ));
        assert!(source.contains("#include \"ksmbd_ida.h\""));
        assert!(source.contains("#include \"../glob.h\""));
        assert!(source.contains("ida_alloc_range(ida, 1, 0xFFFFFFFE"));
        assert!(source.contains("id = ida_alloc_min(ida, 1"));
        assert!(source.contains("if (id == 0xFFFE)"));
        assert!(source.contains("return ida_alloc_min(ida, 1"));
        assert!(source.contains("return ida_alloc(ida"));
        assert!(source.contains("ida_free(ida, id);"));

        let mut ida = KsmbdIda::new();
        assert_eq!(ksmbd_acquire_smb2_tid(&mut ida), Ok(1));
        assert_eq!(ksmbd_acquire_async_msg_id(&mut ida), Ok(2));
        assert_eq!(ksmbd_acquire_id(&mut ida), Ok(0));
        ksmbd_release_id(&mut ida, 1);
        assert_eq!(ksmbd_acquire_smb2_tid(&mut ida), Ok(1));
    }

    #[test]
    fn ksmbd_uid_skips_reserved_lanman_value() {
        let mut ida = KsmbdIda::new();
        for id in 1..SMB2_UID_RESERVED {
            ida.allocated.insert(id);
        }
        assert_eq!(ksmbd_acquire_smb2_uid(&mut ida), Ok(SMB2_UID_RESERVED + 1));
        assert!(ida.contains(SMB2_UID_RESERVED));
        assert!(ida.contains(SMB2_UID_RESERVED + 1));
    }

    #[test]
    fn ksmbd_tid_rejects_all_ones_sentinel() {
        let mut ida = KsmbdIda::new();
        ida.allocated.insert(SMB2_TID_MAX);
        assert_eq!(ida.alloc_range(SMB2_TID_MAX, SMB2_TID_MAX), Err(ENOSPC));
        assert_eq!(ksmbd_acquire_smb2_tid(&mut ida), Ok(SMB2_TID_MIN));
    }
}
