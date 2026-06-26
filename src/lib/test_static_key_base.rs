//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_static_key_base.c
//! test-origin: linux:vendor/linux/lib/test_static_key_base.c
//! Static-key base module state used by jump-label tests.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticKey {
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StaticKeyBaseState {
    pub base_old_true_key: StaticKey,
    pub base_inv_old_true_key: StaticKey,
    pub base_old_false_key: StaticKey,
    pub base_inv_old_false_key: StaticKey,
    pub base_true_key: StaticKey,
    pub base_inv_true_key: StaticKey,
    pub base_false_key: StaticKey,
    pub base_inv_false_key: StaticKey,
}

pub const EXPORTED_STATIC_KEYS: &[&str] = &[
    "base_old_true_key",
    "base_inv_old_true_key",
    "base_old_false_key",
    "base_inv_old_false_key",
    "base_true_key",
    "base_inv_true_key",
    "base_false_key",
    "base_inv_false_key",
];

pub const INITIAL_STATIC_KEYS: StaticKeyBaseState = StaticKeyBaseState {
    base_old_true_key: StaticKey { enabled: true },
    base_inv_old_true_key: StaticKey { enabled: true },
    base_old_false_key: StaticKey { enabled: false },
    base_inv_old_false_key: StaticKey { enabled: false },
    base_true_key: StaticKey { enabled: true },
    base_inv_true_key: StaticKey { enabled: true },
    base_false_key: StaticKey { enabled: false },
    base_inv_false_key: StaticKey { enabled: false },
};

pub const MODULE_AUTHOR: &str = "Jason Baron <jbaron@akamai.com>";
pub const MODULE_DESCRIPTION: &str = "Kernel module to support testing static keys";
pub const MODULE_LICENSE: &str = "GPL";

pub const fn invert_key(key: StaticKey) -> StaticKey {
    StaticKey {
        enabled: !key.enabled,
    }
}

pub const fn test_static_key_base_init(state: StaticKeyBaseState) -> StaticKeyBaseState {
    StaticKeyBaseState {
        base_old_true_key: state.base_old_true_key,
        base_inv_old_true_key: invert_key(state.base_inv_old_true_key),
        base_old_false_key: state.base_old_false_key,
        base_inv_old_false_key: invert_key(state.base_inv_old_false_key),
        base_true_key: state.base_true_key,
        base_inv_true_key: invert_key(state.base_inv_true_key),
        base_false_key: state.base_false_key,
        base_inv_false_key: invert_key(state.base_inv_false_key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_key_base_state_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_static_key_base.c"
        ));
        assert!(source.contains("STATIC_KEY_INIT_TRUE"));
        assert!(source.contains("STATIC_KEY_INIT_FALSE"));
        assert!(source.contains("DEFINE_STATIC_KEY_TRUE(base_true_key);"));
        assert!(source.contains("DEFINE_STATIC_KEY_FALSE(base_false_key);"));
        for symbol in EXPORTED_STATIC_KEYS {
            assert!(source.contains(symbol));
        }
        assert!(source.contains("static_key_disable(key);"));
        assert!(source.contains("static_key_enable(key);"));
        assert!(source.contains("invert_key(&base_inv_old_true_key);"));
        assert!(source.contains("invert_key(&base_inv_false_key.key);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(source.contains(MODULE_DESCRIPTION));

        let initialized = test_static_key_base_init(INITIAL_STATIC_KEYS);
        assert!(initialized.base_old_true_key.enabled);
        assert!(!initialized.base_inv_old_true_key.enabled);
        assert!(!initialized.base_old_false_key.enabled);
        assert!(initialized.base_inv_old_false_key.enabled);
        assert!(initialized.base_true_key.enabled);
        assert!(!initialized.base_inv_true_key.enabled);
        assert!(!initialized.base_false_key.enabled);
        assert!(initialized.base_inv_false_key.enabled);
    }
}
