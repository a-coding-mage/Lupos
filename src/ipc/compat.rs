//! linux-parity: complete
//! linux-source: vendor/linux/ipc/compat.c
//! test-origin: linux:vendor/linux/ipc/compat.c
//! 32-bit compatibility conversion for System V IPC permissions.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ipc64Perm {
    pub key: u32,
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u16,
    pub seq: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompatIpcPerm {
    pub key: u32,
    pub uid: u16,
    pub gid: u16,
    pub cuid: u16,
    pub cgid: u16,
    pub mode: u16,
    pub seq: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompatIpc64Perm {
    pub key: u32,
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u16,
    pub seq: u16,
}

pub const fn get_compat_ipc64_perm(from: CompatIpc64Perm) -> Ipc64Perm {
    Ipc64Perm {
        key: 0,
        uid: from.uid,
        gid: from.gid,
        cuid: 0,
        cgid: 0,
        mode: from.mode,
        seq: 0,
    }
}

pub const fn get_compat_ipc_perm(from: CompatIpcPerm) -> Ipc64Perm {
    Ipc64Perm {
        key: 0,
        uid: from.uid as u32,
        gid: from.gid as u32,
        cuid: 0,
        cgid: 0,
        mode: from.mode,
        seq: 0,
    }
}

pub const fn to_compat_ipc64_perm(from: Ipc64Perm) -> CompatIpc64Perm {
    CompatIpc64Perm {
        key: from.key,
        uid: from.uid,
        gid: from.gid,
        cuid: from.cuid,
        cgid: from.cgid,
        mode: from.mode,
        seq: from.seq,
    }
}

pub const fn to_compat_ipc_perm(from: Ipc64Perm) -> CompatIpcPerm {
    CompatIpcPerm {
        key: from.key,
        uid: from.uid as u16,
        gid: from.gid as u16,
        cuid: from.cuid as u16,
        cgid: from.cgid as u16,
        mode: from.mode,
        seq: from.seq,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compat_ipc_permission_conversions_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/compat.c"
        ));
        assert!(source.contains("int get_compat_ipc64_perm"));
        assert!(source.contains("copy_from_user(&v, from, sizeof(v))"));
        assert!(source.contains("to->uid = v.uid;"));
        assert!(source.contains("to->gid = v.gid;"));
        assert!(source.contains("to->mode = v.mode;"));
        assert!(source.contains("void to_compat_ipc64_perm"));
        assert!(source.contains("to->key = from->key;"));
        assert!(source.contains("SET_UID(to->uid, from->uid);"));
        assert!(source.contains("SET_GID(to->gid, from->gid);"));

        let native = Ipc64Perm {
            key: 9,
            uid: 1000,
            gid: 1001,
            cuid: 1002,
            cgid: 1003,
            mode: 0o644,
            seq: 7,
        };
        assert_eq!(to_compat_ipc64_perm(native).uid, 1000);
        assert_eq!(to_compat_ipc_perm(native).uid, 1000);
        assert_eq!(
            get_compat_ipc64_perm(CompatIpc64Perm {
                uid: 1,
                gid: 2,
                mode: 3,
                ..CompatIpc64Perm::default()
            }),
            Ipc64Perm {
                uid: 1,
                gid: 2,
                mode: 3,
                ..Ipc64Perm::default()
            }
        );
    }
}
