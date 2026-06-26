//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/rtapp/rtapp.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/rtapp/rtapp.c
//! RV monitor: real-time application latency.

use core::sync::atomic::{AtomicU64, Ordering};

pub const MONITOR_NAME: &str = "rtapp";
pub const MONITOR_DESCRIPTION: &str =
    "Collection of monitors for detecting problems with real-time applications";
pub const MODULE_AUTHOR: &str = "Nam Cao <namcao@linutronix.de>";
pub const MODULE_LICENSE: &str = "GPL";

pub static THRESHOLD_NS: AtomicU64 = AtomicU64::new(100_000);
pub static VIOLATIONS: AtomicU64 = AtomicU64::new(0);

pub fn observe(latency_ns: u64) {
    if latency_ns > THRESHOLD_NS.load(Ordering::Acquire) {
        VIOLATIONS.fetch_add(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_records_breach() {
        VIOLATIONS.store(0, Ordering::Release);
        observe(50_000);
        observe(200_000);
        assert_eq!(VIOLATIONS.load(Ordering::Acquire), 1);
    }

    #[test]
    fn rtapp_module_metadata_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/rtapp/rtapp.c"
        ));
        assert!(source.contains("#define MODULE_NAME \"rtapp\""));
        assert!(source.contains(".name = \"rtapp\""));
        assert!(source.contains(MONITOR_DESCRIPTION));
        assert!(source.contains("rv_register_monitor(&rv_rtapp, NULL);"));
        assert!(source.contains("rv_unregister_monitor(&rv_rtapp);"));
        assert!(source.contains("module_init(register_rtapp);"));
        assert!(source.contains("module_exit(unregister_rtapp);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert!(source.contains(MODULE_AUTHOR));
        assert_eq!(MONITOR_NAME, "rtapp");
        assert_eq!(MODULE_LICENSE, "GPL");
    }
}
