//! linux-parity: complete
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! Minimal KUnit-compatible TAP runner.
//!
//! This is not a separate test framework with its own correctness rules. The
//! public output shape follows Linux KUnit TAP so `kunit.py`-style consumers can
//! parse Lupos boot logs while individual cases are ported from `vendor/linux`.

pub struct KunitCase {
    pub domain: &'static str,
    pub suite: &'static str,
    pub name: &'static str,
    pub source: &'static str,
    pub run: fn() -> bool,
}

pub struct KunitSuite {
    pub name: &'static str,
    pub cases: &'static [KunitCase],
}

#[macro_export]
macro_rules! kunit_expect {
    ($cond:expr) => {
        if !$cond {
            return false;
        }
    };
}

#[macro_export]
macro_rules! kunit_assert {
    ($cond:expr) => {
        if !$cond {
            return false;
        }
    };
}

pub const KUNIT_TAP_BANNER: &str = "kunit TAP ok; source-backed cases passed";

pub const DOMAIN_MM: &str = "mm";
pub const DOMAIN_ENTRY: &str = "entry";
pub const DOMAIN_FUTEX: &str = "futex";
pub const DOMAIN_RCU: &str = "rcu";
pub const DOMAIN_FS: &str = "fs";
pub const DOMAIN_IPC: &str = "ipc";
pub const DOMAIN_CGROUP: &str = "cgroup";
pub const DOMAIN_NET: &str = "net";
pub const DOMAIN_DRIVERS: &str = "drivers";
pub const DOMAIN_SECURITY: &str = "security";
pub const DOMAIN_BLOCK: &str = "block";
pub const DOMAIN_IO_URING: &str = "io_uring";

#[cfg(any(
    test,
    feature = "test-kunit",
    feature = "test-mm-kselftests",
    feature = "test-entry-kselftests",
    feature = "test-futex-kselftests",
    feature = "test-rcu-kselftests",
    feature = "test-fs-kselftests",
    feature = "test-ipc-kselftests",
    feature = "test-cgroup-kselftests",
    feature = "test-net-kselftests",
    feature = "test-drivers-kselftests",
    feature = "test-security-kselftests",
    feature = "test-block-kselftests",
    feature = "test-userspace-kselftests",
))]
fn case_groups() -> &'static [&'static [KunitCase]] {
    crate::mm::kunit::CASE_GROUPS
}

#[cfg(not(any(
    test,
    feature = "test-kunit",
    feature = "test-mm-kselftests",
    feature = "test-entry-kselftests",
    feature = "test-futex-kselftests",
    feature = "test-rcu-kselftests",
    feature = "test-fs-kselftests",
    feature = "test-ipc-kselftests",
    feature = "test-cgroup-kselftests",
    feature = "test-net-kselftests",
    feature = "test-drivers-kselftests",
    feature = "test-security-kselftests",
    feature = "test-block-kselftests",
    feature = "test-userspace-kselftests",
)))]
fn case_groups() -> &'static [&'static [KunitCase]] {
    &[]
}

pub fn kunit_case_count() -> usize {
    kunit_case_count_for_domains(&[])
}

pub fn kunit_case_count_for_domains(domains: &[&str]) -> usize {
    case_groups()
        .iter()
        .flat_map(|cases| cases.iter())
        .filter(|case| domains.is_empty() || domains.contains(&case.domain))
        .count()
}

pub fn run_kunit_tap() -> bool {
    run_kunit_tap_for_domains(&[])
}

pub fn run_kunit_tap_for_domains(domains: &[&str]) -> bool {
    crate::linux_driver_abi::tty::serial_println!("KTAP version 1");
    crate::linux_driver_abi::tty::serial_println!("1..1");
    crate::linux_driver_abi::tty::serial_println!("# Subtest: lupos.kunit");
    let total = kunit_case_count_for_domains(domains);
    crate::linux_driver_abi::tty::serial_println!("    1..{}", total);

    if !domains.is_empty() && total == 0 {
        crate::linux_driver_abi::tty::serial_println!("not ok 1 - lupos.kunit");
        return false;
    }

    let mut emitted = 0usize;
    for cases in case_groups() {
        for case in *cases {
            if !domains.is_empty() && !domains.contains(&case.domain) {
                continue;
            }
            emitted += 1;
            if !(case.run)() {
                crate::linux_driver_abi::tty::serial_println!(
                    "    not ok {} - {}.{} # {}",
                    emitted,
                    case.suite,
                    case.name,
                    case.source
                );
                crate::linux_driver_abi::tty::serial_println!("not ok 1 - lupos.kunit");
                return false;
            }
            crate::linux_driver_abi::tty::serial_println!(
                "    ok {} - {}.{} # {}",
                emitted,
                case.suite,
                case.name,
                case.source
            );
        }
    }

    crate::linux_driver_abi::tty::serial_println!("ok 1 - lupos.kunit");
    crate::kernel::printk::log_info!("kunit", "{}", KUNIT_TAP_BANNER);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kunit_case_count_tracks_real_tables() {
        assert_eq!(kunit_case_count(), 5);
        assert_eq!(kunit_case_count_for_domains(&[DOMAIN_MM]), 5);
        for cases in case_groups() {
            for case in *cases {
                assert!(case.source.starts_with("vendor/linux/"));
            }
        }
    }

    #[test]
    fn kunit_cases_pass() {
        for cases in case_groups() {
            for case in *cases {
                assert!((case.run)(), "{}.{}", case.suite, case.name);
            }
        }
    }
}
