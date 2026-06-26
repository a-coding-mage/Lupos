//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfsd/auth.c
//! test-origin: linux:vendor/linux/fs/nfsd/auth.c
//! NFS daemon export flavor and setuser mapping rules.

use crate::include::uapi::errno::ENOMEM;

pub const NFSEXP_ROOTSQUASH: u32 = 0x0004;
pub const NFSEXP_ALLSQUASH: u32 = 0x0008;
pub const GLOBAL_ROOT_ID: u32 = 0;
pub const INVALID_ID: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpFlavorInfo {
    pub pseudoflavor: u32,
    pub flags: u32,
}

pub fn nfsexp_flags(cred_flavor: u32, flavors: &[ExpFlavorInfo], export_flags: u32) -> u32 {
    flavors
        .iter()
        .find(|flavor| flavor.pseudoflavor == cred_flavor)
        .map(|flavor| flavor.flags)
        .unwrap_or(export_flags)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfsdSetuserOutcome {
    pub result: i32,
    pub fsuid: u32,
    pub fsgid: u32,
    pub group_count: usize,
    pub groups_sorted: bool,
    pub root_caps_raised: bool,
}

pub const fn nfsd_setuser_outcome(
    cred_uid: u32,
    cred_gid: u32,
    rq_group_count: usize,
    flags: u32,
    anon_uid: u32,
    anon_gid: u32,
    group_alloc_ok: bool,
) -> NfsdSetuserOutcome {
    let mut fsuid = cred_uid;
    let mut fsgid = cred_gid;
    let mut group_count = rq_group_count;
    let mut groups_sorted = false;

    if flags & NFSEXP_ALLSQUASH != 0 {
        if !group_alloc_ok {
            return NfsdSetuserOutcome {
                result: -ENOMEM,
                fsuid,
                fsgid,
                group_count,
                groups_sorted,
                root_caps_raised: false,
            };
        }
        fsuid = anon_uid;
        fsgid = anon_gid;
        group_count = 0;
    } else if flags & NFSEXP_ROOTSQUASH != 0 {
        if !group_alloc_ok {
            return NfsdSetuserOutcome {
                result: -ENOMEM,
                fsuid,
                fsgid,
                group_count,
                groups_sorted,
                root_caps_raised: false,
            };
        }
        if fsuid == GLOBAL_ROOT_ID {
            fsuid = anon_uid;
        }
        if fsgid == GLOBAL_ROOT_ID {
            fsgid = anon_gid;
        }
        groups_sorted = true;
    }

    if fsuid == INVALID_ID {
        fsuid = anon_uid;
    }
    if fsgid == INVALID_ID {
        fsgid = anon_gid;
    }

    NfsdSetuserOutcome {
        result: 0,
        fsuid,
        fsgid,
        group_count,
        groups_sorted,
        root_caps_raised: fsuid == GLOBAL_ROOT_ID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfsd_auth_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfsd/auth.c"
        ));
        assert!(source.contains("#include <linux/sched.h>"));
        assert!(source.contains("#include \"nfsd.h\""));
        assert!(source.contains("#include \"auth.h\""));
        assert!(source.contains("int nfsexp_flags"));
        assert!(source.contains("for (f = exp->ex_flavors; f < end; f++)"));
        assert!(source.contains("if (f->pseudoflavor == cred->cr_flavor)"));
        assert!(source.contains("return exp->ex_flags;"));
        assert!(source.contains("int nfsd_setuser"));
        assert!(source.contains("put_cred(revert_creds(get_cred(current_real_cred())));"));
        assert!(source.contains("new->fsuid = cred->cr_uid;"));
        assert!(source.contains("new->fsgid = cred->cr_gid;"));
        assert!(source.contains("flags & NFSEXP_ALLSQUASH"));
        assert!(source.contains("groups_alloc(0)"));
        assert!(source.contains("flags & NFSEXP_ROOTSQUASH"));
        assert!(source.contains("uid_eq(new->fsuid, GLOBAL_ROOT_UID)"));
        assert!(source.contains("groups_sort(gi);"));
        assert!(source.contains("uid_eq(new->fsuid, INVALID_UID)"));
        assert!(source.contains("cap_drop_nfsd_set"));
        assert!(source.contains("cap_raise_nfsd_set"));
        assert!(source.contains("return -ENOMEM;"));

        let flavors = [
            ExpFlavorInfo {
                pseudoflavor: 390003,
                flags: NFSEXP_ROOTSQUASH,
            },
            ExpFlavorInfo {
                pseudoflavor: 1,
                flags: NFSEXP_ALLSQUASH,
            },
        ];
        assert_eq!(nfsexp_flags(390003, &flavors, 0), NFSEXP_ROOTSQUASH);
        assert_eq!(nfsexp_flags(2, &flavors, 7), 7);
        let all = nfsd_setuser_outcome(1000, 1000, 3, NFSEXP_ALLSQUASH, 99, 99, true);
        assert_eq!(all.fsuid, 99);
        assert_eq!(all.group_count, 0);
        let root = nfsd_setuser_outcome(0, 0, 2, NFSEXP_ROOTSQUASH, 99, 99, true);
        assert_eq!(root.fsuid, 99);
        assert!(root.groups_sorted);
        assert!(!root.root_caps_raised);
        assert_eq!(
            nfsd_setuser_outcome(0, 0, 2, NFSEXP_ROOTSQUASH, 99, 99, false).result,
            -ENOMEM
        );
        assert!(nfsd_setuser_outcome(0, 0, 1, 0, 99, 99, true).root_caps_raised);
        assert_eq!(
            nfsd_setuser_outcome(INVALID_ID, 10, 0, 0, 99, 99, true).fsuid,
            99
        );
    }
}
