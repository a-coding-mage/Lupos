//! linux-parity: complete
//! linux-source: vendor/linux/fs/drop_caches.c
//! test-origin: linux:vendor/linux/fs/drop_caches.c
//! `/proc/sys/vm/drop_caches` action selection.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DropCachesActions {
    pub drop_pagecache: bool,
    pub drop_slab: bool,
    pub suppress_future_info: bool,
}

pub const DROP_CACHES_SYSCTL_NAME: &str = "drop_caches";
pub const DROP_CACHES_SYSCTL_DIR: &str = "vm";
pub const DROP_CACHES_SYSCTL_MODE: u16 = 0o200;
pub const DROP_CACHES_SYSCTL_MIN: i32 = 1;
pub const DROP_CACHES_SYSCTL_MAX: i32 = 4;

pub const fn drop_caches_actions(value: i32) -> DropCachesActions {
    DropCachesActions {
        drop_pagecache: value & 1 != 0,
        drop_slab: value & 2 != 0,
        suppress_future_info: value & 4 != 0,
    }
}

pub const fn drop_caches_value_in_range(value: i32) -> bool {
    value >= DROP_CACHES_SYSCTL_MIN && value <= DROP_CACHES_SYSCTL_MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_caches_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/drop_caches.c"
        ));
        assert!(source.contains("#include <linux/writeback.h>"));
        assert!(source.contains("#include <linux/sysctl.h>"));
        assert!(source.contains("static int sysctl_drop_caches;"));
        assert!(source.contains("static void drop_pagecache_sb"));
        assert!(source.contains("invalidate_mapping_pages(inode->i_mapping, 0, -1);"));
        assert!(source.contains("cond_resched();"));
        assert!(source.contains("drop_caches_sysctl_handler"));
        assert!(source.contains("proc_dointvec_minmax(table, write, buffer, length, ppos);"));
        assert!(source.contains("if (sysctl_drop_caches & 1)"));
        assert!(source.contains("lru_add_drain_all();"));
        assert!(source.contains("iterate_supers(drop_pagecache_sb, NULL);"));
        assert!(source.contains("count_vm_event(DROP_PAGECACHE);"));
        assert!(source.contains("if (sysctl_drop_caches & 2)"));
        assert!(source.contains("drop_slab();"));
        assert!(source.contains("count_vm_event(DROP_SLAB);"));
        assert!(source.contains("stfu |= sysctl_drop_caches & 4;"));
        assert!(source.contains(".procname\t= \"drop_caches\""));
        assert!(source.contains(".mode\t\t= 0200"));
        assert!(source.contains(".extra1\t\t= SYSCTL_ONE"));
        assert!(source.contains(".extra2\t\t= SYSCTL_FOUR"));
        assert!(source.contains("register_sysctl_init(\"vm\", drop_caches_table);"));
        assert!(source.contains("fs_initcall(init_vm_drop_caches_sysctls);"));

        assert_eq!(
            drop_caches_actions(1),
            DropCachesActions {
                drop_pagecache: true,
                drop_slab: false,
                suppress_future_info: false,
            }
        );
        assert_eq!(
            drop_caches_actions(3),
            DropCachesActions {
                drop_pagecache: true,
                drop_slab: true,
                suppress_future_info: false,
            }
        );
        assert!(drop_caches_actions(4).suppress_future_info);
        assert!(drop_caches_value_in_range(1));
        assert!(drop_caches_value_in_range(4));
        assert!(!drop_caches_value_in_range(0));
        assert!(!drop_caches_value_in_range(5));
    }
}
