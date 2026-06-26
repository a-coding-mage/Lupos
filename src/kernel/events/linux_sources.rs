//! linux-parity: complete
//! linux-source: vendor/linux/kernel/events
//! test-origin: linux:vendor/linux/kernel/events
//! Linux perf event source coverage and unsupported-operation policy.
//!
//! Keep these Linux references source-shaped while routing behavior through
//! `kernel::events::{attr, sys_perf_event_open, perf_event_read_record}`.
//!
//! Refs:
//! - `vendor/linux/kernel/events/{callchain,core,hw_breakpoint,hw_breakpoint_test,ring_buffer,uprobes}.c`

use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

pub const PERF_SOURCE_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerfSourceRole {
    Production,
    KunitTest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportStatus {
    Implemented,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxPerfSource {
    pub path: &'static str,
    pub role: PerfSourceRole,
    pub status: SupportStatus,
    pub unsupported_errno: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxPerfSourceEntry {
    pub path: &'static str,
    pub role: PerfSourceRole,
}

pub const PERF_SOURCES: &[LinuxPerfSourceEntry] = &[
    LinuxPerfSourceEntry {
        path: "vendor/linux/kernel/events/callchain.c",
        role: PerfSourceRole::Production,
    },
    LinuxPerfSourceEntry {
        path: "vendor/linux/kernel/events/core.c",
        role: PerfSourceRole::Production,
    },
    LinuxPerfSourceEntry {
        path: "vendor/linux/kernel/events/hw_breakpoint.c",
        role: PerfSourceRole::Production,
    },
    LinuxPerfSourceEntry {
        path: "vendor/linux/kernel/events/hw_breakpoint_test.c",
        role: PerfSourceRole::KunitTest,
    },
    LinuxPerfSourceEntry {
        path: "vendor/linux/kernel/events/ring_buffer.c",
        role: PerfSourceRole::Production,
    },
    LinuxPerfSourceEntry {
        path: "vendor/linux/kernel/events/uprobes.c",
        role: PerfSourceRole::Production,
    },
];

const IMPLEMENTED_SOURCES: &[&str] = &["vendor/linux/kernel/events/core.c"];

pub fn source_count() -> usize {
    PERF_SOURCES.len()
}

pub fn contains_linux_source(path: &str) -> bool {
    source_entry(path).is_some()
}

pub fn source_policy(path: &'static str) -> LinuxPerfSource {
    let role = source_entry(path)
        .map(|entry| entry.role)
        .unwrap_or(PerfSourceRole::Production);
    let status = if is_implemented(path) {
        SupportStatus::Implemented
    } else {
        SupportStatus::Unsupported
    };
    LinuxPerfSource {
        path,
        role,
        status,
        unsupported_errno: if status == SupportStatus::Unsupported {
            Some(unsupported_errno(path))
        } else {
            None
        },
    }
}

pub fn unsupported_errno(path: &str) -> i32 {
    if contains_linux_source(path) {
        EOPNOTSUPP
    } else {
        ENOENT
    }
}

pub fn all_sources_have_policy() -> Result<(), i32> {
    if source_count() != PERF_SOURCE_COUNT {
        return Err(ENOENT);
    }
    for source in PERF_SOURCES {
        if source.path.is_empty() {
            return Err(ENOENT);
        }
        let policy = source_policy(source.path);
        if policy.path != source.path || policy.role != source.role {
            return Err(ENOENT);
        }
    }
    Ok(())
}

fn source_entry(path: &str) -> Option<&'static LinuxPerfSourceEntry> {
    PERF_SOURCES.iter().find(|source| source.path == path)
}

fn is_implemented(path: &str) -> bool {
    IMPLEMENTED_SOURCES.iter().any(|source| *source == path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

    #[test]
    fn linux_perf_source_inventory_is_complete() {
        assert_eq!(source_count(), PERF_SOURCE_COUNT);
        assert!(contains_linux_source("vendor/linux/kernel/events/core.c"));
        assert!(contains_linux_source(
            "vendor/linux/kernel/events/hw_breakpoint_test.c"
        ));
        assert!(contains_linux_source(
            "vendor/linux/kernel/events/ring_buffer.c"
        ));
        assert_eq!(all_sources_have_policy(), Ok(()));
    }

    #[test]
    fn linux_perf_source_inventory_is_backed_by_vendor_tree() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/events/hw_breakpoint_test.c"
        ));
        assert!(source.contains("KUnit test for hw_breakpoint constraints accounting logic"));
        assert!(source.contains("kunit_test_suites(&hw_breakpoint_test_suite)"));
        assert_eq!(
            source_policy("vendor/linux/kernel/events/hw_breakpoint_test.c").role,
            PerfSourceRole::KunitTest
        );
    }

    #[test]
    fn linux_perf_source_policy_reports_real_support_state() {
        let supported = source_policy("vendor/linux/kernel/events/core.c");
        assert_eq!(supported.status, SupportStatus::Implemented);
        assert_eq!(supported.role, PerfSourceRole::Production);
        assert_eq!(supported.unsupported_errno, None);

        let unsupported = source_policy("vendor/linux/kernel/events/ring_buffer.c");
        assert_eq!(unsupported.status, SupportStatus::Unsupported);
        assert_eq!(unsupported.role, PerfSourceRole::Production);
        assert_eq!(unsupported.unsupported_errno, Some(EOPNOTSUPP));

        let kunit = source_policy("vendor/linux/kernel/events/hw_breakpoint_test.c");
        assert_eq!(kunit.status, SupportStatus::Unsupported);
        assert_eq!(kunit.role, PerfSourceRole::KunitTest);
        assert_eq!(kunit.unsupported_errno, Some(EOPNOTSUPP));

        assert_eq!(
            unsupported_errno("vendor/linux/kernel/events/missing.c"),
            ENOENT
        );
    }
}
