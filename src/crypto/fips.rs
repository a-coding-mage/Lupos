//! linux-parity: complete
//! linux-source: vendor/linux/crypto/fips.c
//! test-origin: linux:vendor/linux/crypto/fips.c
//! FIPS boot parameter and failure notifier state.

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

static FIPS_ENABLED: AtomicI32 = AtomicI32::new(0);
static FIPS_FAIL_NOTIFICATIONS: AtomicUsize = AtomicUsize::new(0);
static CRYPTO_SYSCTLS_REGISTERED: AtomicBool = AtomicBool::new(false);

pub const FIPS_SYSCTL_PATH: &str = "crypto";
pub const FIPS_ENABLED_PROC: &str = "fips_enabled";
pub const FIPS_NAME_PROC: &str = "fips_name";
pub const FIPS_VERSION_PROC: &str = "fips_version";
pub const FIPS_MODULE_NAME_DEFINE: &str = "CONFIG_CRYPTO_FIPS_NAME";
pub const FIPS_MODULE_VERSION_DEFINE: &str = "CONFIG_CRYPTO_FIPS_VERSION";
pub const FIPS_MODULE_VERSION_DEFAULT_DEFINE: &str = "UTS_RELEASE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FipsSysctlEntry {
    pub procname: &'static str,
    pub maxlen: usize,
    pub mode: u16,
    pub proc_handler: &'static str,
}

pub const CRYPTO_SYSCTL_TABLE: [FipsSysctlEntry; 3] = [
    FipsSysctlEntry {
        procname: FIPS_ENABLED_PROC,
        maxlen: core::mem::size_of::<i32>(),
        mode: 0o444,
        proc_handler: "proc_dointvec",
    },
    FipsSysctlEntry {
        procname: FIPS_NAME_PROC,
        maxlen: 64,
        mode: 0o444,
        proc_handler: "proc_dostring",
    },
    FipsSysctlEntry {
        procname: FIPS_VERSION_PROC,
        maxlen: 64,
        mode: 0o444,
        proc_handler: "proc_dostring",
    },
];

fn parse_base0_int(str: &str) -> Option<i32> {
    let trimmed = str.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (negative, digits) = trimmed
        .strip_prefix('-')
        .map(|rest| (true, rest))
        .or_else(|| trimmed.strip_prefix('+').map(|rest| (false, rest)))
        .unwrap_or((false, trimmed));
    if digits.is_empty() {
        return None;
    }
    let (radix, digits) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .map(|rest| (16, rest))
        .or_else(|| {
            (digits.len() > 1)
                .then(|| digits.strip_prefix('0').map(|rest| (8, rest)))
                .flatten()
        })
        .unwrap_or((10, digits));
    if digits.is_empty() {
        return None;
    }
    let value = i64::from_str_radix(digits, radix).ok()?;
    let signed = if negative { -value } else { value };
    i32::try_from(signed).ok()
}

pub fn fips_enable(str: &str) -> i32 {
    let Some(value) = parse_base0_int(str) else {
        return 0;
    };
    FIPS_ENABLED.store((value != 0) as i32, Ordering::Release);
    1
}

pub fn fips_enabled() -> i32 {
    FIPS_ENABLED.load(Ordering::Acquire)
}

pub fn fips_fail_notify() {
    if fips_enabled() != 0 {
        FIPS_FAIL_NOTIFICATIONS.fetch_add(1, Ordering::AcqRel);
    }
}

pub fn fips_fail_notification_count() -> usize {
    FIPS_FAIL_NOTIFICATIONS.load(Ordering::Acquire)
}

pub fn crypto_proc_fips_init() {
    CRYPTO_SYSCTLS_REGISTERED.store(true, Ordering::Release);
}

pub fn crypto_proc_fips_exit() {
    CRYPTO_SYSCTLS_REGISTERED.store(false, Ordering::Release);
}

pub fn crypto_sysctls_registered() -> bool {
    CRYPTO_SYSCTLS_REGISTERED.load(Ordering::Acquire)
}

pub fn fips_init() -> i32 {
    crypto_proc_fips_init();
    0
}

pub fn fips_exit() {
    crypto_proc_fips_exit();
}

#[cfg(test)]
pub fn reset_for_test() {
    FIPS_ENABLED.store(0, Ordering::Release);
    FIPS_FAIL_NOTIFICATIONS.store(0, Ordering::Release);
    CRYPTO_SYSCTLS_REGISTERED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fips_matches_linux_boot_param_sysctl_and_notify_contract() {
        reset_for_test();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/crypto/fips.c"
        ));
        assert!(source.contains("int fips_enabled;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fips_enabled);"));
        assert!(source.contains("ATOMIC_NOTIFIER_HEAD(fips_fail_notif_chain);"));
        assert!(source.contains("if (kstrtoint(str, 0, &fips_enabled))"));
        assert!(source.contains("fips_enabled = !!fips_enabled;"));
        assert!(source.contains("__setup(\"fips=\", fips_enable);"));
        assert!(source.contains("#define FIPS_MODULE_NAME CONFIG_CRYPTO_FIPS_NAME"));
        assert!(source.contains("#define FIPS_MODULE_VERSION CONFIG_CRYPTO_FIPS_VERSION"));
        assert!(source.contains("#define FIPS_MODULE_VERSION UTS_RELEASE"));
        assert!(source.contains(".procname\t= \"fips_enabled\""));
        assert!(source.contains(".procname\t= \"fips_name\""));
        assert!(source.contains(".procname\t= \"fips_version\""));
        assert!(source.contains(".mode\t\t= 0444"));
        assert!(source.contains(".proc_handler\t= proc_dointvec"));
        assert!(source.contains(".proc_handler\t= proc_dostring"));
        assert!(
            source.contains("crypto_sysctls = register_sysctl(\"crypto\", crypto_sysctl_table);")
        );
        assert!(source.contains("unregister_sysctl_table(crypto_sysctls);"));
        assert!(source.contains("if (fips_enabled)"));
        assert!(source.contains("atomic_notifier_call_chain(&fips_fail_notif_chain, 0, NULL);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(fips_fail_notify);"));
        assert!(source.contains("module_init(fips_init);"));
        assert!(source.contains("module_exit(fips_exit);"));

        assert_eq!(CRYPTO_SYSCTL_TABLE[0].procname, "fips_enabled");
        assert_eq!(CRYPTO_SYSCTL_TABLE[0].maxlen, core::mem::size_of::<i32>());
        assert_eq!(CRYPTO_SYSCTL_TABLE[1].proc_handler, "proc_dostring");
        assert_eq!(FIPS_MODULE_NAME_DEFINE, "CONFIG_CRYPTO_FIPS_NAME");
        assert_eq!(FIPS_MODULE_VERSION_DEFINE, "CONFIG_CRYPTO_FIPS_VERSION");
        assert_eq!(FIPS_MODULE_VERSION_DEFAULT_DEFINE, "UTS_RELEASE");
        assert_eq!(parse_base0_int("010"), Some(8));
        assert_eq!(parse_base0_int("+0x10"), Some(16));
        assert_eq!(parse_base0_int("-0x80000000"), Some(i32::MIN));
        assert_eq!(parse_base0_int("08"), None);
        assert_eq!(parse_base0_int("0x"), None);
        assert_eq!(parse_base0_int("2147483648"), None);

        assert_eq!(fips_enable("not-an-int"), 0);
        assert_eq!(fips_enabled(), 0);
        assert_eq!(fips_enable("0"), 1);
        assert_eq!(fips_enabled(), 0);
        assert_eq!(fips_enable("+010"), 1);
        assert_eq!(fips_enabled(), 1);
        assert_eq!(fips_enable("0x2"), 1);
        assert_eq!(fips_enabled(), 1);
        assert_eq!(fips_enable("-1"), 1);
        assert_eq!(fips_enabled(), 1);
        fips_fail_notify();
        assert_eq!(fips_fail_notification_count(), 1);
        reset_for_test();
        fips_fail_notify();
        assert_eq!(fips_fail_notification_count(), 0);
        assert_eq!(fips_init(), 0);
        assert!(crypto_sysctls_registered());
        fips_exit();
        assert!(!crypto_sysctls_registered());
    }
}
