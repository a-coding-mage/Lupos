//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors
//! RV monitor modules.  Each monitor is a small DFA describing a property
//! the kernel must satisfy at runtime.
//!
//! Ref: vendor/linux/kernel/trace/rv/monitors/

pub mod deadline;
pub mod nomiss;
pub mod nrp;
pub mod opid;
pub mod pagefault;
pub mod rtapp;
pub mod sched;
pub mod sco;
pub mod scpd;
pub mod sleep;
pub mod snep;
pub mod snroc;
pub mod sssw;
pub mod stall;
pub mod sts;
pub mod wip;
pub mod wwnr;

pub const MONITOR_MODULES: [&str; 17] = [
    "deadline",
    "nomiss",
    "nrp",
    "opid",
    "pagefault",
    "rtapp",
    "sched",
    "sco",
    "scpd",
    "sleep",
    "snep",
    "snroc",
    "sssw",
    "stall",
    "sts",
    "wip",
    "wwnr",
];

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::string::String;
    use std::vec::Vec;

    #[test]
    fn monitor_module_list_matches_linux_directory() {
        let vendor_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors"
        );
        let mut linux_dirs: Vec<String> = std::fs::read_dir(vendor_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    Some(entry.file_name().to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .collect();
        linux_dirs.sort();

        let mut rust_modules: Vec<String> = MONITOR_MODULES
            .iter()
            .map(|module| String::from(*module))
            .collect();
        rust_modules.sort();

        assert_eq!(rust_modules, linux_dirs);
    }

    #[test]
    fn each_linux_monitor_has_kconfig_and_c_source() {
        let vendor_dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors"
        );

        for module in MONITOR_MODULES {
            let kconfig = std::format!("{vendor_dir}/{module}/Kconfig");
            let c_source = std::format!("{vendor_dir}/{module}/{module}.c");
            assert!(std::path::Path::new(&kconfig).exists(), "{kconfig}");
            assert!(std::path::Path::new(&c_source).exists(), "{c_source}");
        }
    }
}
