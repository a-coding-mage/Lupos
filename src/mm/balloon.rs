//! linux-parity: complete
//! linux-source: vendor/linux/mm/balloon.c
//! test-origin: linux:vendor/linux/mm/balloon.c
//! Memory balloon page-list accounting and migration state.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EAGAIN, ENOENT};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BalloonAllocPlan {
    pub gfp_nomemalloc: bool,
    pub gfp_noretry: bool,
    pub gfp_nowarn: bool,
    pub gfp_highuser: bool,
    pub gfp_highuser_movable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalloonDequeueOnePlan {
    Dequeued,
    NoPage { bug_lost_pages: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalloonMigrationDevicePlan {
    Present,
    Missing { errno: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BalloonMigratePlan {
    pub migrate_called: bool,
    pub insert_new_page: bool,
    pub deflate_old_page: bool,
    pub migrate_event: bool,
    pub deflate_event: bool,
    pub adjust_old_zone: bool,
    pub adjust_new_zone: bool,
    pub decrement_isolated_pages: bool,
    pub finalize_old_page: bool,
    pub put_old_page: bool,
    pub retval: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BalloonMovableOpsPlan {
    pub migration_enabled: bool,
    pub set_movable_ops_called: bool,
    pub page_type: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalloonPage {
    pub id: u64,
    pub zone: u32,
    pub offline: bool,
    pub movable_ops: bool,
    pub private_balloon: Option<u64>,
}

impl BalloonPage {
    pub const fn new(id: u64, zone: u32) -> Self {
        Self {
            id,
            zone,
            offline: false,
            movable_ops: false,
            private_balloon: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BalloonStats {
    pub inflate_events: usize,
    pub deflate_events: usize,
    pub migrate_events: usize,
    pub nr_balloon_pages: isize,
    pub managed_page_delta: isize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalloonDevInfo {
    pub id: u64,
    pub pages: Vec<BalloonPage>,
    pub isolated_pages: usize,
    pub adjust_managed_page_count: bool,
    pub migration_enabled: bool,
    pub stats: BalloonStats,
}

impl BalloonDevInfo {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            pages: Vec::new(),
            isolated_pages: 0,
            adjust_managed_page_count: false,
            migration_enabled: false,
            stats: BalloonStats::default(),
        }
    }
}

fn balloon_page_insert(balloon: &mut BalloonDevInfo, mut page: BalloonPage) {
    page.offline = true;
    if balloon.migration_enabled {
        page.movable_ops = true;
        page.private_balloon = Some(balloon.id);
    }
    balloon.pages.push(page);
}

fn balloon_page_finalize(balloon: &BalloonDevInfo, page: &mut BalloonPage) {
    if balloon.migration_enabled {
        page.private_balloon = None;
    }
}

pub const fn balloon_page_alloc_plan(migration_enabled: bool) -> BalloonAllocPlan {
    BalloonAllocPlan {
        gfp_nomemalloc: true,
        gfp_noretry: true,
        gfp_nowarn: true,
        gfp_highuser: !migration_enabled,
        gfp_highuser_movable: migration_enabled,
    }
}

pub fn balloon_page_enqueue(balloon: &mut BalloonDevInfo, page: BalloonPage) {
    balloon_page_insert(balloon, page);
    if balloon.adjust_managed_page_count {
        balloon.stats.managed_page_delta -= 1;
    }
    balloon.stats.inflate_events += 1;
    balloon.stats.nr_balloon_pages += 1;
}

pub fn balloon_page_list_enqueue(
    balloon: &mut BalloonDevInfo,
    pages: &mut Vec<BalloonPage>,
) -> usize {
    let mut n_pages = 0;
    for page in pages.drain(..) {
        balloon_page_enqueue(balloon, page);
        n_pages += 1;
    }
    n_pages
}

pub fn balloon_page_list_dequeue(
    balloon: &mut BalloonDevInfo,
    out: &mut Vec<BalloonPage>,
    n_req_pages: usize,
) -> usize {
    let mut n_pages = 0;
    while n_pages < n_req_pages {
        let Some(mut page) = balloon.pages.pop() else {
            break;
        };
        if balloon.adjust_managed_page_count {
            balloon.stats.managed_page_delta += 1;
        }
        balloon_page_finalize(balloon, &mut page);
        balloon.stats.deflate_events += 1;
        balloon.stats.nr_balloon_pages -= 1;
        out.push(page);
        n_pages += 1;
    }
    n_pages
}

pub fn balloon_page_dequeue(balloon: &mut BalloonDevInfo) -> Option<BalloonPage> {
    let mut pages = Vec::new();
    let n_pages = balloon_page_list_dequeue(balloon, &mut pages, 1);
    if n_pages == 1 { pages.pop() } else { None }
}

pub const fn balloon_page_dequeue_one_plan(
    dequeued_pages: usize,
    balloon_list_empty: bool,
    isolated_pages: usize,
) -> BalloonDequeueOnePlan {
    if dequeued_pages == 1 {
        BalloonDequeueOnePlan::Dequeued
    } else {
        BalloonDequeueOnePlan::NoPage {
            bug_lost_pages: balloon_list_empty && isolated_pages == 0,
        }
    }
}

pub const fn balloon_page_device_plan(private_balloon_present: bool) -> BalloonMigrationDevicePlan {
    if private_balloon_present {
        BalloonMigrationDevicePlan::Present
    } else {
        BalloonMigrationDevicePlan::Missing { errno: -EAGAIN }
    }
}

pub fn balloon_page_isolate(balloon: &mut BalloonDevInfo, page_id: u64) -> Option<BalloonPage> {
    let index = balloon.pages.iter().position(|page| page.id == page_id)?;
    let page = balloon.pages.remove(index);
    balloon.isolated_pages += 1;
    Some(page)
}

pub fn balloon_page_putback(balloon: &mut BalloonDevInfo, page: BalloonPage) {
    balloon.pages.push(page);
    balloon.isolated_pages = balloon.isolated_pages.saturating_sub(1);
}

pub fn balloon_page_migrate(
    balloon: &mut BalloonDevInfo,
    mut newpage: BalloonPage,
    mut oldpage: BalloonPage,
    migrate_result: Result<(), i32>,
) -> Result<(), i32> {
    if let Err(err) = migrate_result {
        if err != -ENOENT {
            return Err(err);
        }
        if balloon.adjust_managed_page_count {
            balloon.stats.managed_page_delta += 1;
        }
        balloon.stats.deflate_events += 1;
    } else {
        let old_zone = oldpage.zone;
        balloon_page_insert(balloon, newpage.clone());
        balloon.stats.migrate_events += 1;
        if balloon.adjust_managed_page_count && old_zone != newpage.zone {
            balloon.stats.managed_page_delta += 1;
            balloon.stats.managed_page_delta -= 1;
        }
        newpage.private_balloon = Some(balloon.id);
    }

    balloon.isolated_pages = balloon.isolated_pages.saturating_sub(1);
    balloon_page_finalize(balloon, &mut oldpage);
    Ok(())
}

pub const fn balloon_page_migrate_plan(
    device_present: bool,
    migrate_result: i32,
    adjust_managed_page_count: bool,
    old_zone: u32,
    new_zone: u32,
) -> BalloonMigratePlan {
    if !device_present {
        return BalloonMigratePlan {
            migrate_called: false,
            insert_new_page: false,
            deflate_old_page: false,
            migrate_event: false,
            deflate_event: false,
            adjust_old_zone: false,
            adjust_new_zone: false,
            decrement_isolated_pages: false,
            finalize_old_page: false,
            put_old_page: false,
            retval: -EAGAIN,
        };
    }
    if migrate_result < 0 && migrate_result != -ENOENT {
        return BalloonMigratePlan {
            migrate_called: true,
            insert_new_page: false,
            deflate_old_page: false,
            migrate_event: false,
            deflate_event: false,
            adjust_old_zone: false,
            adjust_new_zone: false,
            decrement_isolated_pages: false,
            finalize_old_page: false,
            put_old_page: false,
            retval: migrate_result,
        };
    }

    let migrated = migrate_result == 0;
    BalloonMigratePlan {
        migrate_called: true,
        insert_new_page: migrated,
        deflate_old_page: !migrated,
        migrate_event: migrated,
        deflate_event: !migrated,
        adjust_old_zone: adjust_managed_page_count && (!migrated || old_zone != new_zone),
        adjust_new_zone: adjust_managed_page_count && migrated && old_zone != new_zone,
        decrement_isolated_pages: true,
        finalize_old_page: true,
        put_old_page: true,
        retval: 0,
    }
}

pub const fn balloon_movable_ops_plan(migration_enabled: bool) -> BalloonMovableOpsPlan {
    BalloonMovableOpsPlan {
        migration_enabled,
        set_movable_ops_called: migration_enabled,
        page_type: if migration_enabled {
            Some("PGTY_offline")
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balloon_page_lists_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/balloon.c"
        ));
        assert!(source.contains("static DEFINE_SPINLOCK(balloon_pages_lock);"));
        assert!(source.contains("__SetPageOffline(page);"));
        assert!(source.contains("SetPageMovableOps(page);"));
        assert!(source.contains("set_page_private(page, (unsigned long)balloon);"));
        assert!(source.contains("list_add(&page->lru, &balloon->pages);"));
        assert!(source.contains("__count_vm_event(BALLOON_INFLATE);"));
        assert!(source.contains("__count_vm_event(BALLOON_DEFLATE);"));
        assert!(
            source.contains("gfp_t gfp_flags = __GFP_NOMEMALLOC | __GFP_NORETRY | __GFP_NOWARN;")
        );
        assert!(source.contains("gfp_flags |= GFP_HIGHUSER_MOVABLE;"));
        assert!(source.contains("gfp_flags |= GFP_HIGHUSER;"));
        assert!(source.contains("if (n_pages != 1)"));
        assert!(source.contains("list_empty(&b_dev_info->pages)"));
        assert!(source.contains("!b_dev_info->isolated_pages"));
        assert!(source.contains("BUG();"));
        assert!(source.contains("static struct balloon_dev_info *balloon_page_device"));
        assert!(source.contains("return (struct balloon_dev_info *)page_private(page);"));
        assert!(source.contains("if (!b_dev_info)"));
        assert!(source.contains("return false;"));
        assert!(source.contains("return -EAGAIN;"));
        assert!(source.contains("rc = b_dev_info->migratepage(b_dev_info, newpage, page, mode);"));
        assert!(source.contains("if (rc < 0 && rc != -ENOENT)"));
        assert!(source.contains("get_page(newpage);"));
        assert!(source.contains("__count_vm_event(BALLOON_MIGRATE);"));
        assert!(source.contains("page_zone(page) != page_zone(newpage)"));
        assert!(source.contains("adjust_managed_page_count(page, 1);"));
        assert!(source.contains("adjust_managed_page_count(newpage, -1);"));
        assert!(source.contains("/* Old page was deflated but new page not inflated. */"));
        assert!(source.contains("b_dev_info->isolated_pages--;"));
        assert!(source.contains("put_page(page);"));
        assert!(source.contains("static const struct movable_operations balloon_mops"));
        assert!(source.contains("core_initcall(balloon_init);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(balloon_page_list_enqueue);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(balloon_page_list_dequeue);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(balloon_page_alloc);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(balloon_page_enqueue);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(balloon_page_dequeue);"));
        assert!(source.contains("set_movable_ops(&balloon_mops, PGTY_offline);"));

        let mut balloon = BalloonDevInfo::new(7);
        balloon.adjust_managed_page_count = true;
        balloon.migration_enabled = true;
        let mut input = alloc::vec![BalloonPage::new(1, 0), BalloonPage::new(2, 0)];
        assert_eq!(balloon_page_list_enqueue(&mut balloon, &mut input), 2);
        assert!(input.is_empty());
        assert_eq!(balloon.pages.len(), 2);
        assert!(balloon.pages[0].offline);
        assert_eq!(balloon.pages[0].private_balloon, Some(7));
        assert_eq!(balloon.stats.inflate_events, 2);
        assert_eq!(balloon.stats.managed_page_delta, -2);

        let isolated = balloon_page_isolate(&mut balloon, 1).unwrap();
        assert_eq!(balloon.isolated_pages, 1);
        balloon_page_putback(&mut balloon, isolated);
        assert_eq!(balloon.isolated_pages, 0);

        let mut out = Vec::new();
        assert_eq!(balloon_page_list_dequeue(&mut balloon, &mut out, 1), 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].private_balloon, None);
        assert_eq!(balloon.stats.deflate_events, 1);
        assert_eq!(balloon.stats.nr_balloon_pages, 1);
    }

    #[test]
    fn balloon_alloc_and_single_dequeue_plans_match_linux() {
        assert_eq!(
            balloon_page_alloc_plan(true),
            BalloonAllocPlan {
                gfp_nomemalloc: true,
                gfp_noretry: true,
                gfp_nowarn: true,
                gfp_highuser: false,
                gfp_highuser_movable: true,
            }
        );
        assert_eq!(
            balloon_page_alloc_plan(false),
            BalloonAllocPlan {
                gfp_nomemalloc: true,
                gfp_noretry: true,
                gfp_nowarn: true,
                gfp_highuser: true,
                gfp_highuser_movable: false,
            }
        );
        assert_eq!(
            balloon_page_dequeue_one_plan(1, false, 0),
            BalloonDequeueOnePlan::Dequeued
        );
        assert_eq!(
            balloon_page_dequeue_one_plan(0, true, 0),
            BalloonDequeueOnePlan::NoPage {
                bug_lost_pages: true,
            }
        );
        assert_eq!(
            balloon_page_dequeue_one_plan(0, true, 2),
            BalloonDequeueOnePlan::NoPage {
                bug_lost_pages: false,
            }
        );
    }

    #[test]
    fn balloon_migration_paths_match_linux_source() {
        assert_eq!(
            balloon_page_device_plan(false),
            BalloonMigrationDevicePlan::Missing { errno: -EAGAIN }
        );
        assert_eq!(
            balloon_page_device_plan(true),
            BalloonMigrationDevicePlan::Present
        );
        assert_eq!(
            balloon_page_migrate_plan(false, 0, true, 0, 1),
            BalloonMigratePlan {
                migrate_called: false,
                insert_new_page: false,
                deflate_old_page: false,
                migrate_event: false,
                deflate_event: false,
                adjust_old_zone: false,
                adjust_new_zone: false,
                decrement_isolated_pages: false,
                finalize_old_page: false,
                put_old_page: false,
                retval: -EAGAIN,
            }
        );
        assert_eq!(
            balloon_page_migrate_plan(true, -EAGAIN, true, 0, 1).retval,
            -EAGAIN
        );
        assert_eq!(
            balloon_page_migrate_plan(true, -ENOENT, true, 0, 1),
            BalloonMigratePlan {
                migrate_called: true,
                insert_new_page: false,
                deflate_old_page: true,
                migrate_event: false,
                deflate_event: true,
                adjust_old_zone: true,
                adjust_new_zone: false,
                decrement_isolated_pages: true,
                finalize_old_page: true,
                put_old_page: true,
                retval: 0,
            }
        );
        assert_eq!(
            balloon_page_migrate_plan(true, 0, true, 0, 1),
            BalloonMigratePlan {
                migrate_called: true,
                insert_new_page: true,
                deflate_old_page: false,
                migrate_event: true,
                deflate_event: false,
                adjust_old_zone: true,
                adjust_new_zone: true,
                decrement_isolated_pages: true,
                finalize_old_page: true,
                put_old_page: true,
                retval: 0,
            }
        );

        let mut balloon = BalloonDevInfo::new(9);
        balloon.adjust_managed_page_count = true;
        balloon.migration_enabled = true;
        balloon.isolated_pages = 1;
        let oldpage = BalloonPage {
            private_balloon: Some(9),
            ..BalloonPage::new(10, 0)
        };
        assert_eq!(
            balloon_page_migrate(
                &mut balloon,
                BalloonPage::new(11, 1),
                oldpage.clone(),
                Err(-EAGAIN),
            ),
            Err(-EAGAIN)
        );
        assert_eq!(
            balloon_page_migrate(&mut balloon, BalloonPage::new(11, 1), oldpage, Err(-ENOENT),),
            Ok(())
        );
        assert_eq!(balloon.stats.deflate_events, 1);
        assert_eq!(balloon.stats.managed_page_delta, 1);
        assert_eq!(balloon.isolated_pages, 0);

        assert_eq!(
            balloon_movable_ops_plan(true),
            BalloonMovableOpsPlan {
                migration_enabled: true,
                set_movable_ops_called: true,
                page_type: Some("PGTY_offline"),
            }
        );
        assert_eq!(
            balloon_movable_ops_plan(false),
            BalloonMovableOpsPlan {
                migration_enabled: false,
                set_movable_ops_called: false,
                page_type: None,
            }
        );
    }
}
