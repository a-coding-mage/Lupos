//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_static_keys.c
//! test-origin: linux:vendor/linux/lib/test_static_keys.c
//! Static key test-module state transitions.

pub const MODULE_DESCRIPTION: &str = "Kernel module for testing static keys";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestKey {
    pub name: &'static str,
    pub init_state: bool,
}

pub const STATIC_KEY_TESTS: [TestKey; 18] = [
    TestKey {
        name: "old_true_key",
        init_state: true,
    },
    TestKey {
        name: "old_false_key",
        init_state: false,
    },
    TestKey {
        name: "true_key_static_branch_likely",
        init_state: true,
    },
    TestKey {
        name: "true_key_static_branch_unlikely",
        init_state: true,
    },
    TestKey {
        name: "false_key_static_branch_likely",
        init_state: false,
    },
    TestKey {
        name: "false_key_static_branch_unlikely",
        init_state: false,
    },
    TestKey {
        name: "base_old_true_key",
        init_state: true,
    },
    TestKey {
        name: "base_inv_old_true_key",
        init_state: false,
    },
    TestKey {
        name: "base_old_false_key",
        init_state: false,
    },
    TestKey {
        name: "base_inv_old_false_key",
        init_state: true,
    },
    TestKey {
        name: "base_true_key_static_branch_likely",
        init_state: true,
    },
    TestKey {
        name: "base_true_key_static_branch_unlikely",
        init_state: true,
    },
    TestKey {
        name: "base_inv_true_key_static_branch_likely",
        init_state: false,
    },
    TestKey {
        name: "base_inv_true_key_static_branch_unlikely",
        init_state: false,
    },
    TestKey {
        name: "base_false_key_static_branch_likely",
        init_state: false,
    },
    TestKey {
        name: "base_false_key_static_branch_unlikely",
        init_state: false,
    },
    TestKey {
        name: "base_inv_false_key_static_branch_likely",
        init_state: true,
    },
    TestKey {
        name: "base_inv_false_key_static_branch_unlikely",
        init_state: true,
    },
];

pub fn verify_keys(keys: &[TestKey], invert: bool) -> bool {
    keys.iter().all(|key| {
        (if invert {
            !key.init_state
        } else {
            key.init_state
        }) == branch_result(*key, invert)
    })
}

pub const fn branch_result(key: TestKey, invert: bool) -> bool {
    if invert {
        !key.init_state
    } else {
        key.init_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_keys_matches_linux_original_test_module() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_static_keys.c"
        ));

        assert!(source.contains("struct static_key old_true_key\t= STATIC_KEY_INIT_TRUE;"));
        assert!(source.contains("DEFINE_STATIC_KEY_TRUE(true_key);"));
        assert!(source.contains("DEFINE_STATIC_KEY_FALSE(false_key);"));
        assert!(source.contains("static void invert_key(struct static_key *key)"));
        assert!(
            source.contains("static int verify_keys(struct test_key *keys, int size, bool invert)")
        );
        assert!(source.contains("struct test_key static_key_tests[]"));
        assert!(source.contains("ret = verify_keys(static_key_tests, size, false);"));
        assert!(source.contains("invert_keys(static_key_tests, size);"));
        assert!(source.contains("ret = verify_keys(static_key_tests, size, true);"));
        assert!(source.contains("module_init(test_static_key_init);"));
        assert!(source.contains(MODULE_DESCRIPTION));
        for key in STATIC_KEY_TESTS {
            assert!(source.contains(key.name.split("_static_branch").next().unwrap()));
        }
        assert_eq!(STATIC_KEY_TESTS.len(), 18);
        assert!(verify_keys(&STATIC_KEY_TESTS, false));
        assert!(verify_keys(&STATIC_KEY_TESTS, true));
    }
}
