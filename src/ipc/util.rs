//! linux-parity: partial
//! linux-source: vendor/linux/ipc/util.c
//! test-origin: linux:vendor/linux/ipc/util.c
//! Common System V IPC id, permission, and command-version helpers.

pub const IPC_PRIVATE: i32 = 0;
pub const IPC_CREAT: i32 = 0o1000;
pub const IPC_EXCL: i32 = 0o2000;
pub const IPC_OLD: i32 = 0;
pub const IPC_64: i32 = 0x0100;
pub const S_IRWXUGO: u16 = 0o777;

pub const IPC_INIT_ORDER: &[&str] = &[
    "proc_mkdir(\"sysvipc\")",
    "sem_init",
    "msg_init",
    "shm_init",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcGetRoute {
    NewPrivate,
    PublicMissingNoEntry,
    PublicMissingCreate,
    PublicExistingExclusive,
    PublicExistingCheckPerms,
}

pub const fn ipc_id(index: i32, seq: i32, seq_shift: u32) -> i32 {
    (seq << seq_shift) + index
}

pub const fn next_sequence(idx: i32, last_idx: i32, seq: i32, seq_max: i32) -> i32 {
    if idx <= last_idx {
        let next = seq + 1;
        if next >= seq_max { 0 } else { next }
    } else {
        seq
    }
}

pub const fn ipcget_route(key: i32, flags: i32, key_found: bool) -> IpcGetRoute {
    if key == IPC_PRIVATE {
        return IpcGetRoute::NewPrivate;
    }
    if !key_found {
        if (flags & IPC_CREAT) == 0 {
            IpcGetRoute::PublicMissingNoEntry
        } else {
            IpcGetRoute::PublicMissingCreate
        }
    } else if (flags & IPC_CREAT) != 0 && (flags & IPC_EXCL) != 0 {
        IpcGetRoute::PublicExistingExclusive
    } else {
        IpcGetRoute::PublicExistingCheckPerms
    }
}

pub const fn ipc_permission_allowed(
    object_mode: u16,
    flag: u16,
    owner_match: bool,
    group_match: bool,
    capable_ipc_owner: bool,
) -> bool {
    let requested = (flag >> 6) | (flag >> 3) | flag;
    let granted = if owner_match {
        object_mode >> 6
    } else if group_match {
        object_mode >> 3
    } else {
        object_mode
    };
    ((requested & !granted & 0o7) == 0) || capable_ipc_owner
}

pub const fn ipc_update_mode(current_mode: u16, requested_mode: u16) -> u16 {
    (current_mode & !S_IRWXUGO) | (requested_mode & S_IRWXUGO)
}

pub const fn ipc_parse_version(cmd: &mut i32) -> i32 {
    if (*cmd & IPC_64) != 0 {
        *cmd ^= IPC_64;
        IPC_64
    } else {
        IPC_OLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_util_core_rules_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/util.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/ipc.h"
        ));
        assert!(source.contains("proc_mkdir(\"sysvipc\", NULL);"));
        assert!(source.contains("sem_init();"));
        assert!(source.contains("msg_init();"));
        assert!(source.contains("shm_init();"));
        assert!(source.contains("static const struct rhashtable_params ipc_kht_params"));
        assert!(source.contains("ids->in_use = 0;"));
        assert!(source.contains("ids->max_idx = -1;"));
        assert!(source.contains("if (idx <= ids->last_idx)"));
        assert!(source.contains("new->id = (new->seq << ipcmni_seq_shift()) + idx;"));
        assert!(source.contains("if (params->key == IPC_PRIVATE)"));
        assert!(source.contains("if (!(flg & IPC_CREAT))"));
        assert!(source.contains("if (flg & IPC_CREAT && flg & IPC_EXCL)"));
        assert!(source.contains("requested_mode = (flag >> 6) | (flag >> 3) | flag;"));
        assert!(source.contains("granted_mode >>= 6;"));
        assert!(source.contains("granted_mode >>= 3;"));
        assert!(source.contains("out->mode = (out->mode & ~S_IRWXUGO)"));
        assert!(source.contains("if (*cmd & IPC_64)"));
        assert!(header.contains("#define IPC_CREAT  00001000"));
        assert!(header.contains("#define IPC_EXCL   00002000"));
        assert!(header.contains("#define IPC_64  0x0100"));

        assert_eq!(IPC_INIT_ORDER.len(), 4);
        assert_eq!(ipc_id(7, 3, 15), 98_311);
        assert_eq!(next_sequence(4, 4, 9, 10), 0);
        assert_eq!(next_sequence(5, 4, 9, 10), 9);
        assert_eq!(ipcget_route(IPC_PRIVATE, 0, false), IpcGetRoute::NewPrivate);
        assert_eq!(ipcget_route(7, 0, false), IpcGetRoute::PublicMissingNoEntry);
        assert_eq!(
            ipcget_route(7, IPC_CREAT, false),
            IpcGetRoute::PublicMissingCreate
        );
        assert_eq!(
            ipcget_route(7, IPC_CREAT | IPC_EXCL, true),
            IpcGetRoute::PublicExistingExclusive
        );
        assert!(ipc_permission_allowed(0o640, 0o400, true, false, false));
        assert!(!ipc_permission_allowed(0o640, 0o002, false, false, false));
        assert!(ipc_permission_allowed(0o640, 0o002, false, false, true));
        assert_eq!(ipc_update_mode(0o10_600, 0o777), 0o10_777);
        let mut cmd = IPC_64 | 14;
        assert_eq!(ipc_parse_version(&mut cmd), IPC_64);
        assert_eq!(cmd, 14);
        let mut old = 14;
        assert_eq!(ipc_parse_version(&mut old), IPC_OLD);
        assert_eq!(old, 14);
    }
}
