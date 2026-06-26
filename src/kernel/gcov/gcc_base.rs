//! linux-parity: complete
//! linux-source: vendor/linux/kernel/gcov/gcc_base.c
//! test-origin: linux:vendor/linux/kernel/gcov/gcc_base.c
//! GCC GCOV constructor and merge stubs.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GcovInfo {
    pub filename: &'static str,
    pub version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GcovAction {
    Add,
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GcovInitResult {
    pub version_magic_printed: Option<u32>,
    pub event_emitted: Option<GcovAction>,
    pub linked_count: usize,
}

static GCOV_VERSION: AtomicU32 = AtomicU32::new(0);
static GCOV_INFO_COUNT: AtomicUsize = AtomicUsize::new(0);
static GCOV_EVENTS_ENABLED: AtomicBool = AtomicBool::new(false);
static GCOV_EVENTS_EMITTED: AtomicUsize = AtomicUsize::new(0);

pub fn gcov_enable_events() {
    GCOV_EVENTS_ENABLED.store(true, Ordering::Release);
}

pub fn gcov_info_count() -> usize {
    GCOV_INFO_COUNT.load(Ordering::Acquire)
}

pub fn gcov_events_emitted() -> usize {
    GCOV_EVENTS_EMITTED.load(Ordering::Acquire)
}

pub fn __gcov_init(info: GcovInfo) -> GcovInitResult {
    let version_magic_printed =
        match GCOV_VERSION.compare_exchange(0, info.version, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Some(info.version),
            Err(_) => None,
        };
    let linked_count = GCOV_INFO_COUNT.fetch_add(1, Ordering::AcqRel) + 1;
    let event_emitted = if GCOV_EVENTS_ENABLED.load(Ordering::Acquire) {
        GCOV_EVENTS_EMITTED.fetch_add(1, Ordering::AcqRel);
        Some(GcovAction::Add)
    } else {
        None
    };

    GcovInitResult {
        version_magic_printed,
        event_emitted,
        linked_count,
    }
}

pub fn __gcov_flush() {}

pub fn __gcov_merge_add(_counters: &mut [i64]) {}

pub fn __gcov_merge_single(_counters: &mut [i64]) {}

pub fn __gcov_merge_delta(_counters: &mut [i64]) {}

pub fn __gcov_merge_ior(_counters: &mut [i64]) {}

pub fn __gcov_merge_time_profile(_counters: &mut [i64]) {}

pub fn __gcov_merge_icall_topn(_counters: &mut [i64]) {}

pub fn __gcov_exit() {}

#[cfg(test)]
fn reset_for_test() {
    GCOV_VERSION.store(0, Ordering::Release);
    GCOV_INFO_COUNT.store(0, Ordering::Release);
    GCOV_EVENTS_ENABLED.store(false, Ordering::Release);
    GCOV_EVENTS_EMITTED.store(0, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcc_base_init_and_noop_exports_match_linux_source() {
        reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/gcov/gcc_base.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/gcov/gcov.h"
        ));
        assert!(source.contains("void __gcov_init(struct gcov_info *info)"));
        assert!(source.contains("static unsigned int gcov_version;"));
        assert!(source.contains("gcov_version = gcov_info_version(info);"));
        assert!(source.contains("pr_info(\"version magic: 0x%x\\n\", gcov_version);"));
        assert!(source.contains("gcov_info_link(info);"));
        assert!(source.contains("if (gcov_events_enabled)"));
        assert!(source.contains("gcov_event(GCOV_ADD, info);"));
        assert!(source.contains("void __gcov_flush(void)"));
        assert!(
            source.contains("void __gcov_merge_add(gcov_type *counters, unsigned int n_counters)")
        );
        assert!(source.contains("void __gcov_exit(void)"));
        assert!(header.contains("enum gcov_action"));
        assert!(header.contains("GCOV_ADD"));
        assert!(header.contains("extern int gcov_events_enabled;"));

        let first = __gcov_init(GcovInfo {
            filename: "kernel/a.o",
            version: 0x3430_372a,
        });
        assert_eq!(first.version_magic_printed, Some(0x3430_372a));
        assert_eq!(first.event_emitted, None);
        assert_eq!(first.linked_count, 1);

        gcov_enable_events();
        let second = __gcov_init(GcovInfo {
            filename: "kernel/b.o",
            version: 0x3430_372a,
        });
        assert_eq!(second.version_magic_printed, None);
        assert_eq!(second.event_emitted, Some(GcovAction::Add));
        assert_eq!(gcov_info_count(), 2);
        assert_eq!(gcov_events_emitted(), 1);

        let mut counters = [1, 2, 3];
        __gcov_merge_add(&mut counters);
        __gcov_merge_single(&mut counters);
        __gcov_merge_delta(&mut counters);
        __gcov_merge_ior(&mut counters);
        __gcov_merge_time_profile(&mut counters);
        __gcov_merge_icall_topn(&mut counters);
        assert_eq!(counters, [1, 2, 3]);
        __gcov_flush();
        __gcov_exit();
    }
}
