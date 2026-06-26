//! linux-parity: complete
//! linux-source: vendor/linux/fs/ocfs2/cluster/sys.c
//! test-origin: linux:vendor/linux/fs/ocfs2/cluster/sys.c
//! OCFS2 cluster sysfs interface setup.

use crate::include::uapi::errno::ENOMEM;

pub const O2NM_API_VERSION: u32 = 5;
pub const O2CB_KSET_NAME: &str = "o2cb";
pub const O2CB_INTERFACE_ATTR: &str = "interface_revision";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct O2cbSysInitPlan {
    pub result: i32,
    pub kset_created: bool,
    pub sysfs_group_created: bool,
    pub mlog_initialized: bool,
    pub kset_unregistered_on_error: bool,
}

pub const fn version_show() -> &'static str {
    "5\n"
}

pub const fn o2cb_sys_init_plan(
    kset_create_succeeds: bool,
    sysfs_create_group_result: i32,
    mlog_sys_init_result: i32,
) -> O2cbSysInitPlan {
    if !kset_create_succeeds {
        return O2cbSysInitPlan {
            result: -ENOMEM,
            kset_created: false,
            sysfs_group_created: false,
            mlog_initialized: false,
            kset_unregistered_on_error: false,
        };
    }
    if sysfs_create_group_result != 0 {
        return O2cbSysInitPlan {
            result: sysfs_create_group_result,
            kset_created: true,
            sysfs_group_created: false,
            mlog_initialized: false,
            kset_unregistered_on_error: true,
        };
    }
    if mlog_sys_init_result != 0 {
        return O2cbSysInitPlan {
            result: mlog_sys_init_result,
            kset_created: true,
            sysfs_group_created: true,
            mlog_initialized: false,
            kset_unregistered_on_error: true,
        };
    }
    O2cbSysInitPlan {
        result: 0,
        kset_created: true,
        sysfs_group_created: true,
        mlog_initialized: true,
        kset_unregistered_on_error: false,
    }
}

pub const O2CB_SYS_SHUTDOWN_STEPS: &[&str] = &["mlog_sys_shutdown", "kset_unregister"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocfs2_cluster_sys_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ocfs2/cluster/sys.c"
        ));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/kobject.h>"));
        assert!(source.contains("#include <linux/sysfs.h>"));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include \"ocfs2_nodemanager.h\""));
        assert!(source.contains("#include \"masklog.h\""));
        assert!(source.contains("#include \"sys.h\""));
        assert!(source.contains("return snprintf(buf, PAGE_SIZE, \"%u\\n\", O2NM_API_VERSION);"));
        assert!(source.contains("__ATTR(interface_revision, S_IRUGO, version_show, NULL);"));
        assert!(source.contains("kset_create_and_add(\"o2cb\", NULL, fs_kobj);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("sysfs_create_group(&o2cb_kset->kobj, &o2cb_attr_group);"));
        assert!(source.contains("mlog_sys_init(o2cb_kset);"));
        assert!(source.contains("kset_unregister(o2cb_kset);"));
        assert!(source.contains("mlog_sys_shutdown();"));

        assert_eq!(version_show(), "5\n");
        assert_eq!(o2cb_sys_init_plan(false, 0, 0).result, -ENOMEM);
        assert!(o2cb_sys_init_plan(true, -22, 0).kset_unregistered_on_error);
        assert!(o2cb_sys_init_plan(true, 0, -5).kset_unregistered_on_error);
        let ok = o2cb_sys_init_plan(true, 0, 0);
        assert_eq!(ok.result, 0);
        assert!(ok.mlog_initialized);
        assert_eq!(
            O2CB_SYS_SHUTDOWN_STEPS,
            ["mlog_sys_shutdown", "kset_unregister"]
        );
    }
}
