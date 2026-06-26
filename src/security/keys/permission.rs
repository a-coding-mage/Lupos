//! linux-parity: complete
//! linux-source: vendor/linux/security/keys/permission.c
//! test-origin: linux:vendor/linux/security/keys/permission.c
//! Key permission bit selection and key liveness validation.

use super::{Key, KeyState};
use crate::include::uapi::errno::{EACCES, EKEYEXPIRED, EKEYREVOKED, ENOKEY};

pub const KEY_POS_VIEW: u32 = 0x0100_0000;
pub const KEY_POS_READ: u32 = 0x0200_0000;
pub const KEY_POS_WRITE: u32 = 0x0400_0000;
pub const KEY_POS_SEARCH: u32 = 0x0800_0000;
pub const KEY_POS_LINK: u32 = 0x1000_0000;
pub const KEY_POS_SETATTR: u32 = 0x2000_0000;
pub const KEY_POS_ALL: u32 = 0x3f00_0000;

pub const KEY_USR_VIEW: u32 = 0x0001_0000;
pub const KEY_USR_READ: u32 = 0x0002_0000;
pub const KEY_USR_WRITE: u32 = 0x0004_0000;
pub const KEY_USR_SEARCH: u32 = 0x0008_0000;
pub const KEY_USR_LINK: u32 = 0x0010_0000;
pub const KEY_USR_SETATTR: u32 = 0x0020_0000;
pub const KEY_USR_ALL: u32 = 0x003f_0000;

pub const KEY_GRP_VIEW: u32 = 0x0000_0100;
pub const KEY_GRP_READ: u32 = 0x0000_0200;
pub const KEY_GRP_WRITE: u32 = 0x0000_0400;
pub const KEY_GRP_SEARCH: u32 = 0x0000_0800;
pub const KEY_GRP_LINK: u32 = 0x0000_1000;
pub const KEY_GRP_SETATTR: u32 = 0x0000_2000;
pub const KEY_GRP_ALL: u32 = 0x0000_3f00;

pub const KEY_OTH_VIEW: u32 = 0x0000_0001;
pub const KEY_OTH_READ: u32 = 0x0000_0002;
pub const KEY_OTH_WRITE: u32 = 0x0000_0004;
pub const KEY_OTH_SEARCH: u32 = 0x0000_0008;
pub const KEY_OTH_LINK: u32 = 0x0000_0010;
pub const KEY_OTH_SETATTR: u32 = 0x0000_0020;
pub const KEY_OTH_ALL: u32 = 0x0000_003f;

pub const KEY_FLAG_DEAD: u32 = 0;
pub const KEY_FLAG_REVOKED: u32 = 1;
pub const KEY_FLAG_INVALIDATED: u32 = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyNeedPerm {
    Unspecified,
    View,
    Read,
    Write,
    Search,
    Link,
    Setattr,
    Unlink,
    SysadminOverride,
    AuthtokenOverride,
    DeferPermCheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyCred<'a> {
    pub fsuid: u32,
    pub fsgid: u32,
    pub groups: &'a [u32],
}

pub fn key_task_permission(
    key: &Key,
    possessed: bool,
    cred: KeyCred<'_>,
    need_perm: KeyNeedPerm,
    lsm_result: Result<(), i32>,
) -> Result<(), i32> {
    let Some(mask) = need_perm_mask(need_perm)? else {
        return lsm_result;
    };

    let mut kperm = if key.uid == cred.fsuid {
        key.perm >> 16
    } else if key.perm & KEY_GRP_ALL != 0
        && (key.gid == cred.fsgid || cred.groups.iter().any(|gid| *gid == key.gid))
    {
        key.perm >> 8
    } else {
        key.perm
    };

    if possessed {
        kperm |= key.perm >> 24;
    }

    if (kperm & mask) != mask {
        return Err(-EACCES);
    }

    lsm_result
}

pub const fn key_validate_flags(flags: u64, expiry: Option<i64>, now: i64) -> Result<(), i32> {
    if flags & (1u64 << KEY_FLAG_INVALIDATED) != 0 {
        return Err(-ENOKEY);
    }
    if flags & ((1u64 << KEY_FLAG_REVOKED) | (1u64 << KEY_FLAG_DEAD)) != 0 {
        return Err(-EKEYREVOKED);
    }
    if let Some(expiry) = expiry
        && now >= expiry
    {
        return Err(-EKEYEXPIRED);
    }
    Ok(())
}

pub const fn key_validate_state(
    state: KeyState,
    flags: u64,
    expiry: Option<i64>,
    now: i64,
) -> Result<(), i32> {
    if matches!(state, KeyState::Revoked) {
        return Err(-EKEYREVOKED);
    }
    key_validate_flags(flags, expiry, now)
}

const fn need_perm_mask(need_perm: KeyNeedPerm) -> Result<Option<u32>, i32> {
    match need_perm {
        KeyNeedPerm::View => Ok(Some(KEY_OTH_VIEW)),
        KeyNeedPerm::Read => Ok(Some(KEY_OTH_READ)),
        KeyNeedPerm::Write => Ok(Some(KEY_OTH_WRITE)),
        KeyNeedPerm::Search => Ok(Some(KEY_OTH_SEARCH)),
        KeyNeedPerm::Link => Ok(Some(KEY_OTH_LINK)),
        KeyNeedPerm::Setattr => Ok(Some(KEY_OTH_SETATTR)),
        KeyNeedPerm::Unlink
        | KeyNeedPerm::SysadminOverride
        | KeyNeedPerm::AuthtokenOverride
        | KeyNeedPerm::DeferPermCheck => Ok(None),
        KeyNeedPerm::Unspecified => Err(-EACCES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use alloc::vec::Vec;

    fn key_with_perm(perm: u32) -> Key {
        Key {
            id: 1,
            key_type: String::from("user"),
            description: String::from("k"),
            payload: Vec::new(),
            links: Vec::new(),
            uid: 1000,
            gid: 100,
            perm,
            state: KeyState::Live,
        }
    }

    #[test]
    fn key_task_permission_matches_linux_permission_lanes() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/keys/permission.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/key.h"
        ));
        let bpf_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/bpf/prog_tests/verify_pkcs7_sig.c"
        ));

        assert!(source.contains("case KEY_NEED_VIEW:\tmask = KEY_OTH_VIEW;"));
        assert!(source.contains("WARN_ON(1);"));
        assert!(source.contains("case KEY_NEED_UNLINK:"));
        assert!(source.contains("case KEY_SYSADMIN_OVERRIDE:"));
        assert!(source.contains("goto lsm;"));
        assert!(source.contains("key = key_ref_to_ptr(key_ref);"));
        assert!(source.contains("kperm = key->perm >> 16;"));
        assert!(source.contains("uid_eq(key->uid, cred->fsuid)"));
        assert!(source.contains("gid_valid(key->gid) && key->perm & KEY_GRP_ALL"));
        assert!(source.contains("groups_search(cred->group_info, key->gid);"));
        assert!(source.contains("kperm = key->perm >> 8;"));
        assert!(source.contains("kperm = key->perm;"));
        assert!(source.contains("if (is_key_possessed(key_ref))"));
        assert!(source.contains("if ((kperm & mask) != mask)"));
        assert!(source.contains("return security_key_permission(key_ref, cred, need_perm);"));
        assert!(source.contains("EXPORT_SYMBOL(key_task_permission);"));
        assert!(header.contains("#define KEY_POS_VIEW\t0x01000000"));
        assert!(header.contains("enum key_need_perm"));
        assert!(bpf_selftest.contains("Ensure key_task_permission() is called"));
        assert!(bpf_selftest.contains("0x37373737"));
        assert!(bpf_selftest.contains("0x3f3f3f3f"));

        let user_read = key_with_perm(KEY_USR_READ);
        let owner = KeyCred {
            fsuid: 1000,
            fsgid: 200,
            groups: &[],
        };
        assert_eq!(
            key_task_permission(&user_read, false, owner, KeyNeedPerm::Read, Ok(())),
            Ok(())
        );
        assert_eq!(
            key_task_permission(&user_read, false, owner, KeyNeedPerm::Write, Ok(())),
            Err(-EACCES)
        );

        let group_search = key_with_perm(KEY_GRP_SEARCH);
        let group_member = KeyCred {
            fsuid: 2000,
            fsgid: 300,
            groups: &[100],
        };
        assert_eq!(
            key_task_permission(
                &group_search,
                false,
                group_member,
                KeyNeedPerm::Search,
                Ok(())
            ),
            Ok(())
        );

        let possessed = key_with_perm(KEY_POS_LINK);
        let other = KeyCred {
            fsuid: 2000,
            fsgid: 300,
            groups: &[],
        };
        assert_eq!(
            key_task_permission(&possessed, true, other, KeyNeedPerm::Link, Ok(())),
            Ok(())
        );
        assert_eq!(
            key_task_permission(&possessed, false, other, KeyNeedPerm::Link, Ok(())),
            Err(-EACCES)
        );
        assert_eq!(
            key_task_permission(&possessed, true, other, KeyNeedPerm::Unlink, Err(-EACCES)),
            Err(-EACCES)
        );

        let no_search = key_with_perm(0x3737_3737);
        let all_search = key_with_perm(0x3f3f_3f3f);
        assert_eq!(
            key_task_permission(&no_search, false, owner, KeyNeedPerm::Search, Ok(())),
            Err(-EACCES)
        );
        assert_eq!(
            key_task_permission(&all_search, false, owner, KeyNeedPerm::Search, Ok(())),
            Ok(())
        );
    }

    #[test]
    fn key_validate_matches_linux_flag_and_expiry_order() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/keys/permission.c"
        ));
        let bpf_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/bpf/prog_tests/verify_pkcs7_sig.c"
        ));
        assert!(source.contains("flags & (1 << KEY_FLAG_INVALIDATED)"));
        assert!(source.contains("(1 << KEY_FLAG_REVOKED) |"));
        assert!(source.contains("(1 << KEY_FLAG_DEAD)"));
        assert!(source.contains("ktime_get_real_seconds() >= expiry"));
        assert!(source.contains("EXPORT_SYMBOL(key_validate);"));
        assert!(bpf_selftest.contains("Ensure key_validate() is called"));
        assert!(bpf_selftest.contains("KEYCTL_SET_TIMEOUT"));

        assert_eq!(
            key_validate_flags(1u64 << KEY_FLAG_INVALIDATED, None, 10),
            Err(-ENOKEY)
        );
        assert_eq!(
            key_validate_flags(1u64 << KEY_FLAG_REVOKED, None, 10),
            Err(-EKEYREVOKED)
        );
        assert_eq!(key_validate_flags(0, Some(10), 10), Err(-EKEYEXPIRED));
        assert_eq!(key_validate_flags(0, Some(11), 10), Ok(()));
        assert_eq!(
            key_validate_state(KeyState::Revoked, 0, None, 0),
            Err(-EKEYREVOKED)
        );
    }
}
