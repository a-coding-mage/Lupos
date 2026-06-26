//! linux-parity: complete
//! linux-source: vendor/linux/ipc/ipc_sysctl.c
//! test-origin: linux:vendor/linux/ipc/ipc_sysctl.c
//! SysV IPC sysctl table shape and IPCMNI extension state.

use crate::include::uapi::errno::{ENOMEM, ERANGE};

pub const RADIX_TREE_MAP_SIZE: u32 = 64;
pub const IPCMNI_SHIFT: u32 = 15;
pub const IPCMNI_EXTEND_SHIFT: u32 = 24;
pub const IPCMNI_EXTEND_MIN_CYCLE: u32 = RADIX_TREE_MAP_SIZE * RADIX_TREE_MAP_SIZE;
pub const IPCMNI: u32 = 1 << IPCMNI_SHIFT;
pub const IPCMNI_EXTEND: u32 = 1 << IPCMNI_EXTEND_SHIFT;
pub const SYSCTL_ZERO: &str = "SYSCTL_ZERO";
pub const SYSCTL_ONE: &str = "SYSCTL_ONE";
pub const SYSCTL_INT_MAX: &str = "SYSCTL_INT_MAX";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcSysctlEntry {
    pub procname: &'static str,
    pub namespace_field: Option<&'static str>,
    pub handler: &'static str,
    pub maxlen: &'static str,
    pub mode: u16,
    pub extra1: Option<&'static str>,
    pub extra2: Option<&'static str>,
    pub checkpoint_restore_only: bool,
}

pub const IPC_SYSCTLS: &[IpcSysctlEntry] = &[
    IpcSysctlEntry {
        procname: "shmmax",
        namespace_field: Some("shm_ctlmax"),
        handler: "proc_doulongvec_minmax",
        maxlen: "sizeof(init_ipc_ns.shm_ctlmax)",
        mode: 0o644,
        extra1: None,
        extra2: None,
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "shmall",
        namespace_field: Some("shm_ctlall"),
        handler: "proc_doulongvec_minmax",
        maxlen: "sizeof(init_ipc_ns.shm_ctlall)",
        mode: 0o644,
        extra1: None,
        extra2: None,
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "shmmni",
        namespace_field: Some("shm_ctlmni"),
        handler: "proc_dointvec_minmax",
        maxlen: "sizeof(init_ipc_ns.shm_ctlmni)",
        mode: 0o644,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some("&ipc_mni"),
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "shm_rmid_forced",
        namespace_field: Some("shm_rmid_forced"),
        handler: "proc_ipc_dointvec_minmax_orphans",
        maxlen: "sizeof(init_ipc_ns.shm_rmid_forced)",
        mode: 0o644,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some(SYSCTL_ONE),
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "msgmax",
        namespace_field: Some("msg_ctlmax"),
        handler: "proc_dointvec_minmax",
        maxlen: "sizeof(init_ipc_ns.msg_ctlmax)",
        mode: 0o644,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some(SYSCTL_INT_MAX),
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "msgmni",
        namespace_field: Some("msg_ctlmni"),
        handler: "proc_dointvec_minmax",
        maxlen: "sizeof(init_ipc_ns.msg_ctlmni)",
        mode: 0o644,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some("&ipc_mni"),
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "auto_msgmni",
        namespace_field: None,
        handler: "proc_ipc_auto_msgmni",
        maxlen: "sizeof(int)",
        mode: 0o644,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some(SYSCTL_ONE),
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "msgmnb",
        namespace_field: Some("msg_ctlmnb"),
        handler: "proc_dointvec_minmax",
        maxlen: "sizeof(init_ipc_ns.msg_ctlmnb)",
        mode: 0o644,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some(SYSCTL_INT_MAX),
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "sem",
        namespace_field: Some("sem_ctls"),
        handler: "proc_ipc_sem_dointvec",
        maxlen: "4*sizeof(int)",
        mode: 0o644,
        extra1: None,
        extra2: None,
        checkpoint_restore_only: false,
    },
    IpcSysctlEntry {
        procname: "sem_next_id",
        namespace_field: Some("ids[IPC_SEM_IDS].next_id"),
        handler: "proc_dointvec_minmax",
        maxlen: "sizeof(init_ipc_ns.ids[IPC_SEM_IDS].next_id)",
        mode: 0o444,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some(SYSCTL_INT_MAX),
        checkpoint_restore_only: true,
    },
    IpcSysctlEntry {
        procname: "msg_next_id",
        namespace_field: Some("ids[IPC_MSG_IDS].next_id"),
        handler: "proc_dointvec_minmax",
        maxlen: "sizeof(init_ipc_ns.ids[IPC_MSG_IDS].next_id)",
        mode: 0o444,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some(SYSCTL_INT_MAX),
        checkpoint_restore_only: true,
    },
    IpcSysctlEntry {
        procname: "shm_next_id",
        namespace_field: Some("ids[IPC_SHM_IDS].next_id"),
        handler: "proc_dointvec_minmax",
        maxlen: "sizeof(init_ipc_ns.ids[IPC_SHM_IDS].next_id)",
        mode: 0o444,
        extra1: Some(SYSCTL_ZERO),
        extra2: Some(SYSCTL_INT_MAX),
        checkpoint_restore_only: true,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcMniState {
    pub ipc_mni: u32,
    pub ipc_mni_shift: u32,
    pub ipc_min_cycle: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcSysctlViewer {
    NamespaceRoot,
    NamespaceGroup,
    CheckpointRestoreCapable,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrphanHandlerPlan {
    pub destroy_orphaned: bool,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutoMsgmniPlan {
    pub uses_dummy_value: bool,
    pub log_write_once: bool,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemSysctlPlan {
    pub old_semmni: i32,
    pub sem_check_called: bool,
    pub reset_on_error: bool,
    pub final_semmni: i32,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcOwnership {
    pub uid: u32,
    pub gid: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcSysctlSetupPlan {
    pub sysctl_set_setup: bool,
    pub table_allocated: bool,
    pub rebound_entries: usize,
    pub nulled_entries: usize,
    pub registered: bool,
    pub free_table: bool,
    pub retire_set: bool,
    pub retval: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcSysctlRetirePlan {
    pub unregister_table: bool,
    pub retire_set: bool,
    pub free_table: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IpcSysctlInitPlan {
    pub warn_registration_failed: bool,
    pub retval: i32,
}

pub const fn proc_ipc_dointvec_minmax_orphans_plan(
    proc_result: i32,
    shm_rmid_forced: bool,
) -> OrphanHandlerPlan {
    OrphanHandlerPlan {
        destroy_orphaned: proc_result >= 0 && shm_rmid_forced,
        retval: proc_result,
    }
}

pub const fn proc_ipc_auto_msgmni_plan(write: bool, proc_result: i32) -> AutoMsgmniPlan {
    AutoMsgmniPlan {
        uses_dummy_value: true,
        log_write_once: write,
        retval: proc_result,
    }
}

pub const fn sem_check_semmni(semmni: i32, ipc_mni: u32) -> i32 {
    if semmni < 0 || semmni as u32 > ipc_mni {
        -ERANGE
    } else {
        0
    }
}

pub const fn proc_ipc_sem_dointvec_plan(
    old_semmni: i32,
    new_semmni: i32,
    proc_result: i32,
    ipc_mni: u32,
) -> SemSysctlPlan {
    if proc_result != 0 {
        return SemSysctlPlan {
            old_semmni,
            sem_check_called: false,
            reset_on_error: true,
            final_semmni: old_semmni,
            retval: proc_result,
        };
    }

    let check = sem_check_semmni(new_semmni, ipc_mni);
    SemSysctlPlan {
        old_semmni,
        sem_check_called: true,
        reset_on_error: check != 0,
        final_semmni: if check == 0 { new_semmni } else { old_semmni },
        retval: check,
    }
}

pub const fn ipc_set_ownership_plan(
    ns_root_uid: Option<u32>,
    ns_root_gid: Option<u32>,
) -> IpcOwnership {
    IpcOwnership {
        uid: match ns_root_uid {
            Some(uid) => uid,
            None => 0,
        },
        gid: match ns_root_gid {
            Some(gid) => gid,
            None => 0,
        },
    }
}

pub const fn ipc_permissions(mode: u16, viewer: IpcSysctlViewer, next_id_entry: bool) -> u16 {
    if next_id_entry {
        if let IpcSysctlViewer::CheckpointRestoreCapable = viewer {
            return 0o666;
        }
    }
    let selected = match viewer {
        IpcSysctlViewer::NamespaceRoot => mode >> 6,
        IpcSysctlViewer::NamespaceGroup => mode >> 3,
        IpcSysctlViewer::CheckpointRestoreCapable | IpcSysctlViewer::Other => mode,
    } & 0o7;
    (selected << 6) | (selected << 3) | selected
}

pub fn ipc_rebind_field(procname: &str) -> Option<&'static str> {
    IPC_SYSCTLS
        .iter()
        .find(|entry| entry.procname == procname)
        .and_then(|entry| entry.namespace_field)
}

pub const fn setup_ipc_sysctls_plan(
    kmemdup_succeeds: bool,
    register_succeeds: bool,
) -> IpcSysctlSetupPlan {
    if !kmemdup_succeeds {
        return IpcSysctlSetupPlan {
            sysctl_set_setup: true,
            table_allocated: false,
            rebound_entries: 0,
            nulled_entries: 0,
            registered: false,
            free_table: true,
            retire_set: true,
            retval: false,
        };
    }

    let rebound_entries = IPC_SYSCTLS.len() - 1;
    let nulled_entries = 1;
    IpcSysctlSetupPlan {
        sysctl_set_setup: true,
        table_allocated: true,
        rebound_entries,
        nulled_entries,
        registered: register_succeeds,
        free_table: !register_succeeds,
        retire_set: !register_succeeds,
        retval: register_succeeds,
    }
}

pub const fn retire_ipc_sysctls_plan() -> IpcSysctlRetirePlan {
    IpcSysctlRetirePlan {
        unregister_table: true,
        retire_set: true,
        free_table: true,
    }
}

pub const fn ipc_sysctl_init_plan(setup_succeeds: bool) -> IpcSysctlInitPlan {
    IpcSysctlInitPlan {
        warn_registration_failed: !setup_succeeds,
        retval: if setup_succeeds { 0 } else { -ENOMEM },
    }
}

pub const fn auto_msgmni_write_changes_value(_write: bool) -> bool {
    false
}

pub const fn ipc_mni_extend_state(ipcmni_extend: bool) -> IpcMniState {
    if ipcmni_extend {
        IpcMniState {
            ipc_mni: IPCMNI_EXTEND,
            ipc_mni_shift: IPCMNI_EXTEND_SHIFT,
            ipc_min_cycle: IPCMNI_EXTEND_MIN_CYCLE,
        }
    } else {
        IpcMniState {
            ipc_mni: IPCMNI,
            ipc_mni_shift: IPCMNI_SHIFT,
            ipc_min_cycle: RADIX_TREE_MAP_SIZE,
        }
    }
}

pub const fn ipcmni_idx_mask(state: IpcMniState) -> u32 {
    (1 << state.ipc_mni_shift) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_sysctl_table_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/ipc_sysctl.c"
        ));
        let util = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/util.h"
        ));
        let radix = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/radix-tree.h"
        ));
        assert!(source.contains("static int proc_ipc_dointvec_minmax_orphans"));
        assert!(source.contains("if (ns->shm_rmid_forced)"));
        assert!(source.contains("shm_destroy_orphaned(ns);"));
        assert!(source.contains("static int proc_ipc_auto_msgmni"));
        assert!(source.contains("ipc_table.data = &dummy;"));
        assert!(source.contains("if (write)"));
        assert!(source.contains("writing to auto_msgmni has no effect"));
        assert!(source.contains("static int proc_ipc_sem_dointvec"));
        assert!(source.contains("semmni = ns->sem_ctls[3];"));
        assert!(source.contains("ret = proc_dointvec(table, write, buffer, lenp, ppos);"));
        assert!(source.contains("ret = sem_check_semmni(ns);"));
        assert!(source.contains("ns->sem_ctls[3] = semmni;"));
        assert!(source.contains("static const struct ctl_table ipc_sysctls[]"));
        for name in [
            "shmmax",
            "shmall",
            "shmmni",
            "shm_rmid_forced",
            "msgmax",
            "msgmni",
            "auto_msgmni",
            "msgmnb",
            "sem",
        ] {
            assert!(source.contains(name));
        }
        assert!(source.contains("sem_next_id"));
        assert!(source.contains("msg_next_id"));
        assert!(source.contains("shm_next_id"));
        assert!(source.contains(".mode\t\t= 0644"));
        assert!(source.contains(".mode\t\t= 0444"));
        assert!(source.contains(".extra2\t\t= &ipc_mni"));
        assert!(source.contains("static struct ctl_table_set *set_lookup"));
        assert!(source.contains("static int set_is_seen"));
        assert!(source.contains("static void ipc_set_ownership"));
        assert!(source.contains("make_kuid(ns->user_ns, 0);"));
        assert!(source.contains("static int ipc_permissions"));
        assert!(source.contains("checkpoint_restore_ns_capable_noaudit"));
        assert!(source.contains("return (mode << 6) | (mode << 3) | mode;"));
        assert!(source.contains("bool setup_ipc_sysctls(struct ipc_namespace *ns)"));
        assert!(source.contains("setup_sysctl_set(&ns->ipc_set, &set_root, set_is_seen);"));
        assert!(source.contains("tbl = kmemdup(ipc_sysctls, sizeof(ipc_sysctls), GFP_KERNEL);"));
        assert!(source.contains("tbl[i].data = &ns->shm_ctlmax;"));
        assert!(source.contains("tbl[i].data = NULL;"));
        assert!(source.contains("__register_sysctl_table(&ns->ipc_set, \"kernel\", tbl,"));
        assert!(source.contains("kfree(tbl);"));
        assert!(source.contains("retire_sysctl_set(&ns->ipc_set);"));
        assert!(source.contains("void retire_ipc_sysctls(struct ipc_namespace *ns)"));
        assert!(source.contains("unregister_sysctl_table(ns->ipc_sysctls);"));
        assert!(source.contains("device_initcall(ipc_sysctl_init);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("early_param(\"ipcmni_extend\", ipc_mni_extend);"));
        assert!(source.contains("ipc_mni = IPCMNI_EXTEND;"));
        assert!(source.contains("ipc_min_cycle = IPCMNI_EXTEND_MIN_CYCLE;"));
        assert!(util.contains("#define IPCMNI_SHIFT\t\t15"));
        assert!(util.contains("#define IPCMNI_EXTEND_SHIFT\t24"));
        assert!(util.contains(
            "#define IPCMNI_EXTEND_MIN_CYCLE\t(RADIX_TREE_MAP_SIZE * RADIX_TREE_MAP_SIZE)"
        ));
        assert!(util.contains("#define IPCMNI\t\t\t(1 << IPCMNI_SHIFT)"));
        assert!(util.contains("#define IPCMNI_EXTEND\t\t(1 << IPCMNI_EXTEND_SHIFT)"));
        assert!(radix.contains("#define RADIX_TREE_MAP_SIZE\t(1UL << RADIX_TREE_MAP_SHIFT)"));

        assert_eq!(IPCMNI, 32_768);
        assert_eq!(IPCMNI_EXTEND, 16_777_216);
        assert_eq!(IPCMNI_EXTEND_MIN_CYCLE, 4096);
        assert_eq!(IPC_SYSCTLS.len(), 12);
        assert_eq!(ipc_rebind_field("msgmnb"), Some("msg_ctlmnb"));
        assert_eq!(ipc_rebind_field("auto_msgmni"), None);
        assert_eq!(
            IPC_SYSCTLS
                .iter()
                .find(|entry| entry.procname == "sem_next_id")
                .map(|entry| (entry.mode, entry.extra1, entry.extra2)),
            Some((0o444, Some(SYSCTL_ZERO), Some(SYSCTL_INT_MAX)))
        );
        assert_eq!(
            IPC_SYSCTLS
                .iter()
                .find(|entry| entry.procname == "shmmni")
                .map(|entry| entry.extra2),
            Some(Some("&ipc_mni"))
        );
        assert!(!auto_msgmni_write_changes_value(true));
        assert_eq!(
            ipc_permissions(0o444, IpcSysctlViewer::CheckpointRestoreCapable, true),
            0o666
        );
        assert_eq!(
            ipc_permissions(0o640, IpcSysctlViewer::NamespaceGroup, false),
            0o444
        );
        assert_eq!(ipc_mni_extend_state(false).ipc_mni, 32_768);
        assert_eq!(
            ipc_mni_extend_state(false).ipc_min_cycle,
            RADIX_TREE_MAP_SIZE
        );
        assert_eq!(ipc_mni_extend_state(true).ipc_mni_shift, 24);
        assert_eq!(
            ipc_mni_extend_state(true).ipc_min_cycle,
            IPCMNI_EXTEND_MIN_CYCLE
        );
        assert_eq!(ipcmni_idx_mask(ipc_mni_extend_state(false)), 0x7fff);
    }

    #[test]
    fn ipc_sysctl_handlers_match_linux_side_effects() {
        assert_eq!(
            proc_ipc_dointvec_minmax_orphans_plan(-ENOMEM, true),
            OrphanHandlerPlan {
                destroy_orphaned: false,
                retval: -ENOMEM,
            }
        );
        assert_eq!(
            proc_ipc_dointvec_minmax_orphans_plan(0, true),
            OrphanHandlerPlan {
                destroy_orphaned: true,
                retval: 0,
            }
        );
        assert_eq!(
            proc_ipc_auto_msgmni_plan(true, 0),
            AutoMsgmniPlan {
                uses_dummy_value: true,
                log_write_once: true,
                retval: 0,
            }
        );
        assert_eq!(sem_check_semmni(-1, IPCMNI), -ERANGE);
        assert_eq!(sem_check_semmni(IPCMNI as i32 + 1, IPCMNI), -ERANGE);
        assert_eq!(sem_check_semmni(IPCMNI as i32, IPCMNI), 0);

        assert_eq!(
            proc_ipc_sem_dointvec_plan(128, 256, -ENOMEM, IPCMNI),
            SemSysctlPlan {
                old_semmni: 128,
                sem_check_called: false,
                reset_on_error: true,
                final_semmni: 128,
                retval: -ENOMEM,
            }
        );
        assert_eq!(
            proc_ipc_sem_dointvec_plan(128, IPCMNI as i32 + 1, 0, IPCMNI),
            SemSysctlPlan {
                old_semmni: 128,
                sem_check_called: true,
                reset_on_error: true,
                final_semmni: 128,
                retval: -ERANGE,
            }
        );
        assert_eq!(
            proc_ipc_sem_dointvec_plan(128, 256, 0, IPCMNI),
            SemSysctlPlan {
                old_semmni: 128,
                sem_check_called: true,
                reset_on_error: false,
                final_semmni: 256,
                retval: 0,
            }
        );
    }

    #[test]
    fn ipc_sysctl_registration_lifecycle_matches_linux() {
        assert_eq!(
            ipc_set_ownership_plan(Some(1000), None),
            IpcOwnership { uid: 1000, gid: 0 }
        );
        assert_eq!(
            setup_ipc_sysctls_plan(false, true),
            IpcSysctlSetupPlan {
                sysctl_set_setup: true,
                table_allocated: false,
                rebound_entries: 0,
                nulled_entries: 0,
                registered: false,
                free_table: true,
                retire_set: true,
                retval: false,
            }
        );
        assert_eq!(
            setup_ipc_sysctls_plan(true, false),
            IpcSysctlSetupPlan {
                sysctl_set_setup: true,
                table_allocated: true,
                rebound_entries: 11,
                nulled_entries: 1,
                registered: false,
                free_table: true,
                retire_set: true,
                retval: false,
            }
        );
        assert_eq!(
            setup_ipc_sysctls_plan(true, true),
            IpcSysctlSetupPlan {
                sysctl_set_setup: true,
                table_allocated: true,
                rebound_entries: 11,
                nulled_entries: 1,
                registered: true,
                free_table: false,
                retire_set: false,
                retval: true,
            }
        );
        assert_eq!(
            retire_ipc_sysctls_plan(),
            IpcSysctlRetirePlan {
                unregister_table: true,
                retire_set: true,
                free_table: true,
            }
        );
        assert_eq!(
            ipc_sysctl_init_plan(false),
            IpcSysctlInitPlan {
                warn_registration_failed: true,
                retval: -ENOMEM,
            }
        );
        assert_eq!(
            ipc_sysctl_init_plan(true),
            IpcSysctlInitPlan {
                warn_registration_failed: false,
                retval: 0,
            }
        );
    }
}
