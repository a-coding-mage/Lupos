//! linux-parity: complete
//! linux-source: vendor/linux/security/keys/sysctl.c
//! test-origin: linux:vendor/linux/security/keys/sysctl.c
//! `/proc/sys/kernel/keys` quota controls.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::include::uapi::errno::{EINVAL, ENOENT};

pub const KEY_QUOTA_MAXKEYS_DEFAULT: u32 = 200;
pub const KEY_QUOTA_MAXBYTES_DEFAULT: u32 = 20_000;
pub const KEY_QUOTA_ROOT_MAXKEYS_DEFAULT: u32 = 1_000_000;
pub const KEY_QUOTA_ROOT_MAXBYTES_DEFAULT: u32 = 25_000_000;
pub const KEY_GC_DELAY_DEFAULT: u32 = 5 * 60;
pub const PERSISTENT_KEYRING_EXPIRY_DEFAULT: u32 = 3 * 24 * 3600;

static REGISTERED: AtomicBool = AtomicBool::new(false);
static KEY_QUOTA_MAXKEYS: AtomicU32 = AtomicU32::new(KEY_QUOTA_MAXKEYS_DEFAULT);
static KEY_QUOTA_MAXBYTES: AtomicU32 = AtomicU32::new(KEY_QUOTA_MAXBYTES_DEFAULT);
static KEY_QUOTA_ROOT_MAXKEYS: AtomicU32 = AtomicU32::new(KEY_QUOTA_ROOT_MAXKEYS_DEFAULT);
static KEY_QUOTA_ROOT_MAXBYTES: AtomicU32 = AtomicU32::new(KEY_QUOTA_ROOT_MAXBYTES_DEFAULT);
static KEY_GC_DELAY: AtomicU32 = AtomicU32::new(KEY_GC_DELAY_DEFAULT);
static PERSISTENT_KEYRING_EXPIRY: AtomicU32 = AtomicU32::new(PERSISTENT_KEYRING_EXPIRY_DEFAULT);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeySysctl {
    pub procname: &'static str,
    pub mode: u16,
    pub min: u32,
    pub max: u32,
}

pub const KEY_SYSCTLS: [KeySysctl; 6] = [
    KeySysctl {
        procname: "maxkeys",
        mode: 0o644,
        min: 1,
        max: i32::MAX as u32,
    },
    KeySysctl {
        procname: "maxbytes",
        mode: 0o644,
        min: 1,
        max: i32::MAX as u32,
    },
    KeySysctl {
        procname: "root_maxkeys",
        mode: 0o644,
        min: 1,
        max: i32::MAX as u32,
    },
    KeySysctl {
        procname: "root_maxbytes",
        mode: 0o644,
        min: 1,
        max: i32::MAX as u32,
    },
    KeySysctl {
        procname: "gc_delay",
        mode: 0o644,
        min: 0,
        max: i32::MAX as u32,
    },
    KeySysctl {
        procname: "persistent_keyring_expiry",
        mode: 0o644,
        min: 0,
        max: i32::MAX as u32,
    },
];

pub fn init_security_keys_sysctls() -> i32 {
    REGISTERED.store(true, Ordering::Release);
    0
}

pub fn registered() -> bool {
    REGISTERED.load(Ordering::Acquire)
}

pub fn read_sysctl(name: &str) -> Result<u32, i32> {
    match name {
        "maxkeys" => Ok(KEY_QUOTA_MAXKEYS.load(Ordering::Acquire)),
        "maxbytes" => Ok(KEY_QUOTA_MAXBYTES.load(Ordering::Acquire)),
        "root_maxkeys" => Ok(KEY_QUOTA_ROOT_MAXKEYS.load(Ordering::Acquire)),
        "root_maxbytes" => Ok(KEY_QUOTA_ROOT_MAXBYTES.load(Ordering::Acquire)),
        "gc_delay" => Ok(KEY_GC_DELAY.load(Ordering::Acquire)),
        "persistent_keyring_expiry" => Ok(PERSISTENT_KEYRING_EXPIRY.load(Ordering::Acquire)),
        _ => Err(-ENOENT),
    }
}

pub fn write_sysctl(name: &str, value: u32) -> Result<(), i32> {
    let Some(entry) = KEY_SYSCTLS.iter().find(|entry| entry.procname == name) else {
        return Err(-ENOENT);
    };
    if value < entry.min || value > entry.max {
        return Err(-EINVAL);
    }
    match name {
        "maxkeys" => KEY_QUOTA_MAXKEYS.store(value, Ordering::Release),
        "maxbytes" => KEY_QUOTA_MAXBYTES.store(value, Ordering::Release),
        "root_maxkeys" => KEY_QUOTA_ROOT_MAXKEYS.store(value, Ordering::Release),
        "root_maxbytes" => KEY_QUOTA_ROOT_MAXBYTES.store(value, Ordering::Release),
        "gc_delay" => KEY_GC_DELAY.store(value, Ordering::Release),
        "persistent_keyring_expiry" => PERSISTENT_KEYRING_EXPIRY.store(value, Ordering::Release),
        _ => return Err(-ENOENT),
    }
    Ok(())
}

#[cfg(test)]
pub fn reset_for_test() {
    REGISTERED.store(false, Ordering::Release);
    KEY_QUOTA_MAXKEYS.store(KEY_QUOTA_MAXKEYS_DEFAULT, Ordering::Release);
    KEY_QUOTA_MAXBYTES.store(KEY_QUOTA_MAXBYTES_DEFAULT, Ordering::Release);
    KEY_QUOTA_ROOT_MAXKEYS.store(KEY_QUOTA_ROOT_MAXKEYS_DEFAULT, Ordering::Release);
    KEY_QUOTA_ROOT_MAXBYTES.store(KEY_QUOTA_ROOT_MAXBYTES_DEFAULT, Ordering::Release);
    KEY_GC_DELAY.store(KEY_GC_DELAY_DEFAULT, Ordering::Release);
    PERSISTENT_KEYRING_EXPIRY.store(PERSISTENT_KEYRING_EXPIRY_DEFAULT, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_sysctls_register_linux_kernel_keys_table() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        reset_for_test();

        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/security/keys/sysctl.c"
        ));
        assert!(source.contains(".procname = \"maxkeys\""));
        assert!(source.contains(".procname = \"root_maxbytes\""));
        assert!(source.contains(".procname = \"gc_delay\""));
        assert!(source.contains("register_sysctl_init(\"kernel/keys\", key_sysctls)"));
        assert!(source.contains("early_initcall(init_security_keys_sysctls)"));

        assert_eq!(init_security_keys_sysctls(), 0);
        assert!(registered());
        assert_eq!(read_sysctl("maxkeys"), Ok(200));
        assert_eq!(read_sysctl("maxbytes"), Ok(20_000));
        assert_eq!(read_sysctl("root_maxkeys"), Ok(1_000_000));
        assert_eq!(read_sysctl("root_maxbytes"), Ok(25_000_000));
        assert_eq!(read_sysctl("gc_delay"), Ok(300));
        assert_eq!(read_sysctl("persistent_keyring_expiry"), Ok(259_200));

        assert_eq!(write_sysctl("maxkeys", 1), Ok(()));
        assert_eq!(read_sysctl("maxkeys"), Ok(1));
        assert_eq!(write_sysctl("maxkeys", 0), Err(-EINVAL));
        assert_eq!(write_sysctl("gc_delay", 0), Ok(()));
        assert_eq!(write_sysctl("missing", 1), Err(-ENOENT));
    }
}
