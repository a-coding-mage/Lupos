//! linux-parity: complete
//! linux-source: vendor/linux/mm/cma_sysfs.c
//! test-origin: linux:vendor/linux/mm/cma_sysfs.c
//! CMA sysfs allocation/release counters.

extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::include::uapi::errno::ENOMEM;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmaSysfsArea {
    pub name: String,
    pub count: usize,
    pub available_count: usize,
    pub nr_pages_succeeded: u64,
    pub nr_pages_failed: u64,
    pub nr_pages_released: u64,
    pub kobject_allocated: bool,
    pub kobject_registered: bool,
}

impl CmaSysfsArea {
    pub fn new(name: &str, count: usize, available_count: usize) -> Self {
        Self {
            name: String::from(name),
            count,
            available_count,
            nr_pages_succeeded: 0,
            nr_pages_failed: 0,
            nr_pages_released: 0,
            kobject_allocated: false,
            kobject_registered: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CmaSysfsInitReport {
    pub result: Result<Vec<String>, i32>,
    pub root_created: bool,
    pub root_released: bool,
    pub released_on_error: Vec<String>,
}

pub fn cma_sysfs_account_success_pages(cma: &mut CmaSysfsArea, nr_pages: u64) {
    cma.nr_pages_succeeded = cma.nr_pages_succeeded.saturating_add(nr_pages);
}

pub fn cma_sysfs_account_fail_pages(cma: &mut CmaSysfsArea, nr_pages: u64) {
    cma.nr_pages_failed = cma.nr_pages_failed.saturating_add(nr_pages);
}

pub fn cma_sysfs_account_release_pages(cma: &mut CmaSysfsArea, nr_pages: u64) {
    cma.nr_pages_released = cma.nr_pages_released.saturating_add(nr_pages);
}

pub fn alloc_pages_success_show(cma: &CmaSysfsArea) -> String {
    format!("{}\n", cma.nr_pages_succeeded)
}

pub fn alloc_pages_fail_show(cma: &CmaSysfsArea) -> String {
    format!("{}\n", cma.nr_pages_failed)
}

pub fn release_pages_success_show(cma: &CmaSysfsArea) -> String {
    format!("{}\n", cma.nr_pages_released)
}

pub fn total_pages_show(cma: &CmaSysfsArea) -> String {
    format!("{}\n", cma.count)
}

pub fn available_pages_show(cma: &CmaSysfsArea) -> String {
    format!("{}\n", cma.available_count)
}

pub fn cma_sysfs_attrs() -> [&'static str; 5] {
    [
        "alloc_pages_success",
        "alloc_pages_fail",
        "release_pages_success",
        "total_pages",
        "available_pages",
    ]
}

pub fn cma_sysfs_init(areas: &mut [CmaSysfsArea]) -> Result<Vec<String>, i32> {
    cma_sysfs_init_report(areas, true, None, None).result
}

pub fn cma_sysfs_init_report(
    areas: &mut [CmaSysfsArea],
    root_available: bool,
    allocation_fail_at: Option<usize>,
    kobject_add_error_at: Option<(usize, i32)>,
) -> CmaSysfsInitReport {
    let mut report = CmaSysfsInitReport {
        result: Err(-ENOMEM),
        root_created: false,
        root_released: false,
        released_on_error: Vec::new(),
    };

    if !root_available {
        return report;
    }
    report.root_created = true;

    let mut registered = Vec::new();
    for i in 0..areas.len() {
        if allocation_fail_at == Some(i) {
            cleanup_registered_areas(areas, i, &mut report.released_on_error);
            report.root_released = true;
            return report;
        }

        areas[i].kobject_allocated = true;
        if let Some((fail_index, err)) = kobject_add_error_at {
            if fail_index == i {
                report.result = Err(err);
                report.released_on_error.push(areas[i].name.clone());
                cma_kobj_release(&mut areas[i]);
                cleanup_registered_areas(areas, i, &mut report.released_on_error);
                report.root_released = true;
                return report;
            }
        }

        areas[i].kobject_registered = true;
        registered.push(areas[i].name.clone());
    }

    report.result = Ok(registered);
    report
}

pub fn cma_kobj_release(cma: &mut CmaSysfsArea) {
    cma.kobject_allocated = false;
    cma.kobject_registered = false;
}

fn cleanup_registered_areas(
    areas: &mut [CmaSysfsArea],
    upto_exclusive: usize,
    released: &mut Vec<String>,
) {
    let mut i = upto_exclusive;
    while i > 0 {
        i -= 1;
        if areas[i].kobject_allocated || areas[i].kobject_registered {
            released.push(areas[i].name.clone());
            cma_kobj_release(&mut areas[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cma_sysfs_counters_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/cma_sysfs.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/cma.h"
        ));
        assert!(source.contains("void cma_sysfs_account_success_pages"));
        assert!(source.contains("atomic64_add(nr_pages, &cma->nr_pages_succeeded);"));
        assert!(source.contains("atomic64_add(nr_pages, &cma->nr_pages_failed);"));
        assert!(source.contains("atomic64_add(nr_pages, &cma->nr_pages_released);"));
        assert!(source.contains("return sysfs_emit(buf, \"%llu\\n\""));
        assert!(source.contains("return sysfs_emit(buf, \"%lu\\n\", cma->count);"));
        assert!(source.contains("return sysfs_emit(buf, \"%lu\\n\", cma->available_count);"));
        assert!(source.contains("static struct attribute *cma_attrs[]"));
        assert!(source.contains("static inline struct cma *cma_from_kobj"));
        assert!(source.contains("static const struct kobj_type cma_ktype"));
        assert!(source.contains(".release = cma_kobj_release"));
        assert!(source.contains("kobject_create_and_add(\"cma\", mm_kobj);"));
        assert!(source.contains("cma_kobj = kzalloc_obj(*cma_kobj);"));
        assert!(source.contains("cma->cma_kobj = cma_kobj;"));
        assert!(source.contains("kobject_init_and_add(&cma_kobj->kobj, &cma_ktype"));
        assert!(source.contains("while (--i >= 0)"));
        assert!(source.contains("kobject_put(cma_kobj_root);"));
        assert!(source.contains("subsys_initcall(cma_sysfs_init);"));
        assert!(header.contains("struct cma_kobject"));
        assert!(header.contains("atomic64_t nr_pages_succeeded;"));

        let mut area = CmaSysfsArea::new("unit", 1024, 768);
        cma_sysfs_account_success_pages(&mut area, 10);
        cma_sysfs_account_fail_pages(&mut area, 2);
        cma_sysfs_account_release_pages(&mut area, 7);
        assert_eq!(alloc_pages_success_show(&area), "10\n");
        assert_eq!(alloc_pages_fail_show(&area), "2\n");
        assert_eq!(release_pages_success_show(&area), "7\n");
        assert_eq!(total_pages_show(&area), "1024\n");
        assert_eq!(available_pages_show(&area), "768\n");
        assert!(cma_sysfs_attrs().contains(&"available_pages"));

        let registered = cma_sysfs_init(core::slice::from_mut(&mut area)).unwrap();
        assert_eq!(registered, alloc::vec![String::from("unit")]);
        assert!(area.kobject_allocated);
        assert!(area.kobject_registered);
        cma_kobj_release(&mut area);
        assert!(!area.kobject_allocated);
        assert!(!area.kobject_registered);
    }

    #[test]
    fn cma_sysfs_init_failures_cleanup_like_linux_source() {
        let mut areas = [
            CmaSysfsArea::new("a", 10, 9),
            CmaSysfsArea::new("b", 20, 18),
            CmaSysfsArea::new("c", 30, 27),
        ];

        let report = cma_sysfs_init_report(&mut areas, false, None, None);
        assert_eq!(report.result, Err(-ENOMEM));
        assert!(!report.root_created);

        let report = cma_sysfs_init_report(&mut areas, true, Some(1), None);
        assert_eq!(report.result, Err(-ENOMEM));
        assert!(report.root_created);
        assert!(report.root_released);
        assert_eq!(report.released_on_error, alloc::vec![String::from("a")]);
        assert!(!areas[0].kobject_registered);

        let mut areas = [
            CmaSysfsArea::new("a", 10, 9),
            CmaSysfsArea::new("b", 20, 18),
            CmaSysfsArea::new("c", 30, 27),
        ];
        let report = cma_sysfs_init_report(&mut areas, true, None, Some((2, -5)));
        assert_eq!(report.result, Err(-5));
        assert!(report.root_released);
        assert_eq!(
            report.released_on_error,
            alloc::vec![String::from("c"), String::from("b"), String::from("a")]
        );
        assert!(areas.iter().all(|area| !area.kobject_allocated));
    }
}
