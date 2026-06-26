//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv
//! test-origin: linux:vendor/linux/kernel/trace/rv
//! Runtime-Verification (RV) framework — a small DFA-based verifier that
//! observes the kernel via tracepoints and signals violations.
//!
//! Ref: vendor/linux/kernel/trace/rv/

pub mod monitors;
pub mod reactor_panic;
pub mod reactor_printk;
pub mod rv;
pub mod rv_reactors;

pub const RV_MODULES: [&str; 5] = [
    "monitors",
    "reactor_panic",
    "reactor_printk",
    "rv",
    "rv_reactors",
];
pub const RV_SOURCE_FILES: [&str; 8] = [
    "Kconfig",
    "Makefile",
    "reactor_panic.c",
    "reactor_printk.c",
    "rv.c",
    "rv.h",
    "rv_reactors.c",
    "rv_trace.h",
];
pub const RV_KCONFIG_SYMBOLS: [&str; 4] =
    ["RV", "RV_REACTORS", "RV_REACT_PRINTK", "RV_REACT_PANIC"];

#[cfg(test)]
mod tests {
    use super::*;

    const KCONFIG: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/kernel/trace/rv/Kconfig"
    ));
    const MAKEFILE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/kernel/trace/rv/Makefile"
    ));
    const RV_TRACE_H: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/kernel/trace/rv/rv_trace.h"
    ));

    #[test]
    fn rv_wrapper_matches_linux_source_set() {
        assert_eq!(
            RV_MODULES,
            [
                "monitors",
                "reactor_panic",
                "reactor_printk",
                "rv",
                "rv_reactors"
            ]
        );
        assert_eq!(
            RV_SOURCE_FILES,
            [
                "Kconfig",
                "Makefile",
                "reactor_panic.c",
                "reactor_printk.c",
                "rv.c",
                "rv.h",
                "rv_reactors.c",
                "rv_trace.h"
            ]
        );
        assert_eq!(
            RV_KCONFIG_SYMBOLS,
            ["RV", "RV_REACTORS", "RV_REACT_PRINTK", "RV_REACT_PANIC"]
        );
    }

    #[test]
    fn rv_wrapper_tracks_linux_makefile_and_kconfig() {
        assert!(KCONFIG.contains("menuconfig RV"));
        assert!(KCONFIG.contains("select TRACING"));
        assert!(KCONFIG.contains("config RV_REACTORS"));
        assert!(KCONFIG.contains("config RV_REACT_PRINTK"));
        assert!(KCONFIG.contains("config RV_REACT_PANIC"));
        assert!(KCONFIG.contains("source \"kernel/trace/rv/monitors/wip/Kconfig\""));
        assert!(KCONFIG.contains("source \"kernel/trace/rv/monitors/nomiss/Kconfig\""));

        assert!(MAKEFILE.contains("ccflags-y += -I $(src)"));
        assert!(MAKEFILE.contains("obj-$(CONFIG_RV) += rv.o"));
        assert!(MAKEFILE.contains("obj-$(CONFIG_RV_REACTORS) += rv_reactors.o"));
        assert!(MAKEFILE.contains("obj-$(CONFIG_RV_REACT_PRINTK) += reactor_printk.o"));
        assert!(MAKEFILE.contains("obj-$(CONFIG_RV_REACT_PANIC) += reactor_panic.o"));
        assert!(MAKEFILE.contains("obj-$(CONFIG_RV_MON_WIP) += monitors/wip/wip.o"));
        assert!(MAKEFILE.contains("obj-$(CONFIG_RV_MON_NOMISS) += monitors/nomiss/nomiss.o"));

        assert!(RV_TRACE_H.contains("TRACE_EVENT"));
    }

    #[test]
    fn rv_wrapper_reexports_child_contracts() {
        assert_eq!(monitors::MONITOR_MODULES.len(), 17);

        let before_monitors = rv::count();
        let monitor_name = "rv_wrapper_reexports_child_contracts";
        rv::register(monitor_name);
        rv::enable(monitor_name).unwrap();
        rv::violation(monitor_name);
        assert_eq!(rv::count(), before_monitors + 1);

        let before_reactors = rv_reactors::count();
        rv_reactors::register(rv_reactors::Reactor {
            name: "printk",
            react: reactor_printk::react,
        });
        assert_eq!(rv_reactors::count(), before_reactors + 1);

        let before_panic =
            reactor_panic::PANIC_INVOCATIONS.load(core::sync::atomic::Ordering::Acquire);
        // `reactor_panic::react` diverges by design (Linux vpanic). Catch the
        // unwind so the invocation-count contract below is reachable.
        extern crate std;
        let reaction = std::panic::catch_unwind(|| reactor_panic::react(monitor_name));
        assert!(reaction.is_err());
        assert_eq!(
            reactor_panic::PANIC_INVOCATIONS.load(core::sync::atomic::Ordering::Acquire),
            before_panic + 1
        );
    }
}
