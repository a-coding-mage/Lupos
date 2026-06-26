//! linux-parity: complete
//! linux-source: vendor/linux/mm
//! test-origin: linux:vendor/linux/mm
//! Crate-wide test lock for synchronizing tests that touch global hardware/buddy state.

#[cfg(test)]
extern crate std;

#[cfg(test)]
pub static GLOBAL_HW_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(
    not(test),
    any(
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
    )
))]
pub static GLOBAL_HW_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());
