//! linux-parity: complete
//! linux-source: vendor/linux/ipc/mq_sysctl.c
//! test-origin: linux:vendor/linux/ipc/mq_sysctl.c
//! POSIX message-queue sysctl table shape and namespace ownership rules.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MqSysctlEntry {
    pub procname: &'static str,
    pub init_field: &'static str,
    pub namespace_field: &'static str,
    pub maxlen: usize,
    pub mode: u16,
    pub handler: &'static str,
    pub extra1: Option<&'static str>,
    pub extra2: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MqSysctlBoundEntry {
    pub procname: &'static str,
    pub data: Option<&'static str>,
    pub maxlen: usize,
    pub mode: u16,
    pub handler: &'static str,
    pub extra1: Option<&'static str>,
    pub extra2: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MqSysctlSetState {
    pub initialized: bool,
    pub retired: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MqSysctlHeader {
    pub path: &'static str,
    pub entry_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MqSysctlNamespace {
    pub mq_set: MqSysctlSetState,
    pub mq_sysctls: Option<MqSysctlHeader>,
    pub ctl_table_arg: Option<[MqSysctlBoundEntry; MQ_SYSCTL_COUNT]>,
    pub freed_tables: usize,
    pub unregistered_tables: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MqNamespaceRoot {
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

pub const GLOBAL_ROOT_UID: u32 = 0;
pub const GLOBAL_ROOT_GID: u32 = 0;
pub const MIN_MSGMAX: i32 = 1;
pub const HARD_MSGMAX: i32 = 65_536;
pub const MIN_MSGSIZEMAX: i32 = 128;
pub const HARD_MSGSIZEMAX: i32 = 16 * 1024 * 1024;
pub const MQ_SYSCTL_COUNT: usize = 5;
pub const MQ_SYSCTL_PATH: &str = "fs/mqueue";

pub const MQ_SYSCTLS: [MqSysctlEntry; MQ_SYSCTL_COUNT] = [
    MqSysctlEntry {
        procname: "queues_max",
        init_field: "init_ipc_ns.mq_queues_max",
        namespace_field: "mq_queues_max",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        handler: "proc_dointvec",
        extra1: None,
        extra2: None,
    },
    MqSysctlEntry {
        procname: "msg_max",
        init_field: "init_ipc_ns.mq_msg_max",
        namespace_field: "mq_msg_max",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("msg_max_limit_min"),
        extra2: Some("msg_max_limit_max"),
    },
    MqSysctlEntry {
        procname: "msgsize_max",
        init_field: "init_ipc_ns.mq_msgsize_max",
        namespace_field: "mq_msgsize_max",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("msg_maxsize_limit_min"),
        extra2: Some("msg_maxsize_limit_max"),
    },
    MqSysctlEntry {
        procname: "msg_default",
        init_field: "init_ipc_ns.mq_msg_default",
        namespace_field: "mq_msg_default",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("msg_max_limit_min"),
        extra2: Some("msg_max_limit_max"),
    },
    MqSysctlEntry {
        procname: "msgsize_default",
        init_field: "init_ipc_ns.mq_msgsize_default",
        namespace_field: "mq_msgsize_default",
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o644,
        handler: "proc_dointvec_minmax",
        extra1: Some("msg_maxsize_limit_min"),
        extra2: Some("msg_maxsize_limit_max"),
    },
];

impl Default for MqSysctlNamespace {
    fn default() -> Self {
        Self {
            mq_set: MqSysctlSetState {
                initialized: false,
                retired: false,
            },
            mq_sysctls: None,
            ctl_table_arg: None,
            freed_tables: 0,
            unregistered_tables: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SysctlViewer {
    NamespaceRoot,
    NamespaceGroup,
    Other,
}

pub const fn set_lookup(current_ipc_namespace: u64) -> u64 {
    current_ipc_namespace
}

pub const fn set_is_seen(current_ipc_namespace: u64, set_ipc_namespace: u64) -> bool {
    current_ipc_namespace == set_ipc_namespace
}

pub const fn mq_set_ownership(root: MqNamespaceRoot) -> (u32, u32) {
    let uid = match root.uid {
        Some(uid) => uid,
        None => GLOBAL_ROOT_UID,
    };
    let gid = match root.gid {
        Some(gid) => gid,
        None => GLOBAL_ROOT_GID,
    };
    (uid, gid)
}

pub fn mq_rebind_field(procname: &str) -> Option<&'static str> {
    MQ_SYSCTLS
        .iter()
        .find(|entry| entry.procname == procname)
        .map(|entry| entry.namespace_field)
}

pub fn mq_rebind_data(init_field: &str) -> Option<&'static str> {
    MQ_SYSCTLS
        .iter()
        .find(|entry| entry.init_field == init_field)
        .map(|entry| entry.namespace_field)
}

fn bind_entry(entry: MqSysctlEntry) -> MqSysctlBoundEntry {
    MqSysctlBoundEntry {
        procname: entry.procname,
        data: mq_rebind_data(entry.init_field),
        maxlen: entry.maxlen,
        mode: entry.mode,
        handler: entry.handler,
        extra1: entry.extra1,
        extra2: entry.extra2,
    }
}

pub fn mq_rebound_table() -> [MqSysctlBoundEntry; MQ_SYSCTL_COUNT] {
    [
        bind_entry(MQ_SYSCTLS[0]),
        bind_entry(MQ_SYSCTLS[1]),
        bind_entry(MQ_SYSCTLS[2]),
        bind_entry(MQ_SYSCTLS[3]),
        bind_entry(MQ_SYSCTLS[4]),
    ]
}

pub const fn mq_permissions(mode: u16, viewer: SysctlViewer) -> u16 {
    let selected = match viewer {
        SysctlViewer::NamespaceRoot => mode >> 6,
        SysctlViewer::NamespaceGroup => mode >> 3,
        SysctlViewer::Other => mode,
    } & 0o7;
    (selected << 6) | (selected << 3) | selected
}

pub fn setup_mq_sysctls(
    ns: &mut MqSysctlNamespace,
    allocation_available: bool,
    register_available: bool,
) -> bool {
    ns.mq_set = MqSysctlSetState {
        initialized: true,
        retired: false,
    };
    ns.mq_sysctls = None;
    ns.ctl_table_arg = None;

    let tbl = if allocation_available {
        Some(mq_rebound_table())
    } else {
        None
    };

    if let Some(table) = tbl {
        if register_available {
            ns.mq_sysctls = Some(MqSysctlHeader {
                path: MQ_SYSCTL_PATH,
                entry_count: MQ_SYSCTL_COUNT,
            });
            ns.ctl_table_arg = Some(table);
        } else {
            ns.freed_tables += 1;
        }
    }

    if ns.mq_sysctls.is_none() {
        ns.mq_set.retired = true;
        return false;
    }

    true
}

pub fn retire_mq_sysctls(ns: &mut MqSysctlNamespace) {
    if ns.mq_sysctls.take().is_some() {
        ns.unregistered_tables += 1;
    }
    ns.mq_set.retired = true;
    if ns.ctl_table_arg.take().is_some() {
        ns.freed_tables += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mq_sysctl_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/mq_sysctl.c"
        ));
        assert!(source.contains("static const struct ctl_table mq_sysctls[]"));
        assert!(source.contains(".procname\t= \"queues_max\""));
        assert!(source.contains(".procname\t= \"msg_max\""));
        assert!(source.contains(".procname\t= \"msgsize_max\""));
        assert!(source.contains(".procname\t= \"msg_default\""));
        assert!(source.contains(".procname\t= \"msgsize_default\""));
        assert!(source.contains("mode >>= 6;"));
        assert!(source.contains("mode >>= 3;"));
        assert!(source.contains("return (mode << 6) | (mode << 3) | mode;"));
        assert!(source.contains("static struct ctl_table_set *set_lookup"));
        assert!(source.contains("static int set_is_seen"));
        assert!(source.contains("static void mq_set_ownership"));
        assert!(source.contains("uid_valid(ns_root_uid) ? ns_root_uid : GLOBAL_ROOT_UID"));
        assert!(source.contains("setup_sysctl_set(&ns->mq_set, &set_root, set_is_seen);"));
        assert!(source.contains("tbl = kmemdup(mq_sysctls, sizeof(mq_sysctls), GFP_KERNEL);"));
        assert!(source.contains("__register_sysctl_table(&ns->mq_set"));
        assert!(source.contains("kfree(tbl);"));
        assert!(source.contains("retire_sysctl_set(&ns->mq_set);"));
        assert!(source.contains("unregister_sysctl_table(ns->mq_sysctls);"));

        assert_eq!(MQ_SYSCTLS.len(), 5);
        assert_eq!(MIN_MSGMAX, 1);
        assert_eq!(HARD_MSGMAX, 65_536);
        assert_eq!(MIN_MSGSIZEMAX, 128);
        assert_eq!(HARD_MSGSIZEMAX, 16 * 1024 * 1024);
        assert_eq!(mq_rebind_field("msg_max"), Some("mq_msg_max"));
        assert_eq!(
            mq_rebind_data("init_ipc_ns.mq_msgsize_default"),
            Some("mq_msgsize_default")
        );
        assert_eq!(mq_rebind_data("foreign_table.data"), None);
        assert_eq!(mq_rebind_field("missing"), None);
        assert_eq!(set_lookup(7), 7);
        assert!(set_is_seen(9, 9));
        assert!(!set_is_seen(9, 10));
        assert_eq!(
            mq_set_ownership(MqNamespaceRoot {
                uid: Some(1000),
                gid: None
            }),
            (1000, GLOBAL_ROOT_GID)
        );
        assert_eq!(mq_permissions(0o644, SysctlViewer::NamespaceRoot), 0o666);
        assert_eq!(mq_permissions(0o640, SysctlViewer::NamespaceGroup), 0o444);
        assert_eq!(mq_permissions(0o641, SysctlViewer::Other), 0o111);
    }

    #[test]
    fn setup_rebinds_registers_and_retires_mq_sysctls() {
        let mut ns = MqSysctlNamespace::default();

        assert!(setup_mq_sysctls(&mut ns, true, true));
        assert_eq!(
            ns.mq_sysctls,
            Some(MqSysctlHeader {
                path: MQ_SYSCTL_PATH,
                entry_count: MQ_SYSCTL_COUNT,
            })
        );
        let table = ns.ctl_table_arg.expect("registered table");
        assert_eq!(table[0].data, Some("mq_queues_max"));
        assert_eq!(table[1].extra1, Some("msg_max_limit_min"));
        assert_eq!(table[2].extra2, Some("msg_maxsize_limit_max"));
        assert!(ns.mq_set.initialized);
        assert!(!ns.mq_set.retired);

        retire_mq_sysctls(&mut ns);
        assert_eq!(ns.mq_sysctls, None);
        assert_eq!(ns.ctl_table_arg, None);
        assert!(ns.mq_set.retired);
        assert_eq!(ns.unregistered_tables, 1);
        assert_eq!(ns.freed_tables, 1);
    }

    #[test]
    fn setup_failure_retire_paths_match_linux_cleanup() {
        let mut alloc_failed = MqSysctlNamespace::default();
        assert!(!setup_mq_sysctls(&mut alloc_failed, false, true));
        assert!(alloc_failed.mq_set.initialized);
        assert!(alloc_failed.mq_set.retired);
        assert_eq!(alloc_failed.freed_tables, 0);

        let mut register_failed = MqSysctlNamespace::default();
        assert!(!setup_mq_sysctls(&mut register_failed, true, false));
        assert!(register_failed.mq_set.initialized);
        assert!(register_failed.mq_set.retired);
        assert_eq!(register_failed.mq_sysctls, None);
        assert_eq!(register_failed.ctl_table_arg, None);
        assert_eq!(register_failed.freed_tables, 1);
    }
}
