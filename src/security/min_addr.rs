//! linux-parity: complete
//! linux-source: vendor/linux/security/min_addr.c
//! test-origin: linux:vendor/linux/security/min_addr.c
//! `vm.mmap_min_addr` LSM/DAC floor calculation.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::include::uapi::errno::EPERM;
use crate::kernel::capability::{CAP_SYS_RAWIO, capable};

pub const DEFAULT_MMAP_MIN_ADDR: u64 = 65_536;
pub const LSM_MMAP_MIN_ADDR: u64 = 65_536;

static DAC_MMAP_MIN_ADDR: AtomicU64 = AtomicU64::new(DEFAULT_MMAP_MIN_ADDR);
static MMAP_MIN_ADDR: AtomicU64 = AtomicU64::new(DEFAULT_MMAP_MIN_ADDR);

pub fn update_mmap_min_addr() -> u64 {
    let value = DAC_MMAP_MIN_ADDR
        .load(Ordering::Acquire)
        .max(LSM_MMAP_MIN_ADDR);
    MMAP_MIN_ADDR.store(value, Ordering::Release);
    value
}

pub fn mmap_min_addr() -> u64 {
    MMAP_MIN_ADDR.load(Ordering::Acquire)
}

pub fn dac_mmap_min_addr() -> u64 {
    DAC_MMAP_MIN_ADDR.load(Ordering::Acquire)
}

pub fn set_dac_mmap_min_addr(value: u64) -> u64 {
    DAC_MMAP_MIN_ADDR.store(value, Ordering::Release);
    update_mmap_min_addr()
}

pub fn mmap_min_addr_handler(write: bool, new_value: Option<u64>) -> Result<u64, i32> {
    mmap_min_addr_handler_with_capability(write, new_value, capable(CAP_SYS_RAWIO))
}

pub fn mmap_min_addr_handler_with_capability(
    write: bool,
    new_value: Option<u64>,
    has_cap_sys_rawio: bool,
) -> Result<u64, i32> {
    if write && !has_cap_sys_rawio {
        return Err(-EPERM);
    }
    if write && let Some(value) = new_value {
        DAC_MMAP_MIN_ADDR.store(value, Ordering::Release);
    }
    Ok(update_mmap_min_addr())
}

#[cfg(test)]
pub fn reset_for_test() {
    DAC_MMAP_MIN_ADDR.store(DEFAULT_MMAP_MIN_ADDR, Ordering::Release);
    MMAP_MIN_ADDR.store(DEFAULT_MMAP_MIN_ADDR, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    #[test]
    fn min_addr_handler_updates_dac_then_lsm_maximum() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        let _guard = TEST_LOCK.lock();
        reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/min_addr.c"
        ));
        assert!(source.contains("unsigned long mmap_min_addr;"));
        assert!(source.contains("dac_mmap_min_addr = CONFIG_DEFAULT_MMAP_MIN_ADDR"));
        assert!(
            source.contains("mmap_min_addr = umax(dac_mmap_min_addr, CONFIG_LSM_MMAP_MIN_ADDR);")
        );
        assert!(source.contains("if (write && !capable(CAP_SYS_RAWIO))"));
        assert!(source.contains("pure_initcall(mmap_min_addr_init);"));

        assert_eq!(mmap_min_addr(), DEFAULT_MMAP_MIN_ADDR);
        assert_eq!(
            mmap_min_addr_handler_with_capability(true, Some(32_768), false),
            Err(-EPERM)
        );
        assert_eq!(
            mmap_min_addr_handler_with_capability(true, Some(32_768), true),
            Ok(LSM_MMAP_MIN_ADDR)
        );
        assert_eq!(dac_mmap_min_addr(), 32_768);
        assert_eq!(mmap_min_addr(), LSM_MMAP_MIN_ADDR);
        assert_eq!(
            mmap_min_addr_handler_with_capability(true, Some(131_072), true),
            Ok(131_072)
        );
        assert_eq!(set_dac_mmap_min_addr(4096), LSM_MMAP_MIN_ADDR);
    }
}
