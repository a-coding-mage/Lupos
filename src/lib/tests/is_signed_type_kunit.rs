//! linux-parity: complete
//! linux-source: vendor/linux/lib/tests/is_signed_type_kunit.c
//! test-origin: linux:vendor/linux/lib/tests/is_signed_type_kunit.c
//! KUnit coverage for is_signed_type().

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IsSignedCase {
    pub type_name: &'static str,
    pub is_signed: bool,
}

pub const IS_SIGNED_TYPE_CASES: &[IsSignedCase] = &[
    IsSignedCase {
        type_name: "bool",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "signed char",
        is_signed: true,
    },
    IsSignedCase {
        type_name: "unsigned char",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "char",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "int",
        is_signed: true,
    },
    IsSignedCase {
        type_name: "unsigned int",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "long",
        is_signed: true,
    },
    IsSignedCase {
        type_name: "unsigned long",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "long long",
        is_signed: true,
    },
    IsSignedCase {
        type_name: "unsigned long long",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "enum unsigned_enum",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "enum signed_enum",
        is_signed: true,
    },
    IsSignedCase {
        type_name: "void *",
        is_signed: false,
    },
    IsSignedCase {
        type_name: "const char *",
        is_signed: false,
    },
];

pub fn is_signed_type_case(type_name: &str) -> Option<bool> {
    IS_SIGNED_TYPE_CASES
        .iter()
        .find(|case| case.type_name == type_name)
        .map(|case| case.is_signed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_signed_type_kunit_cases_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/tests/is_signed_type_kunit.c"
        ));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(bool), false);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(signed char), true);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(unsigned char), false);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(char), false);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(int), true);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(unsigned int), false);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(long), true);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(unsigned long), false);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(long long), true);"));
        assert!(
            source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(unsigned long long), false);")
        );
        assert!(
            source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(enum unsigned_enum), false);")
        );
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(enum signed_enum), true);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(void *), false);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, is_signed_type(const char *), false);"));
        assert!(source.contains(".name = \"is_signed_type\""));

        assert_eq!(IS_SIGNED_TYPE_CASES.len(), 14);
        assert_eq!(is_signed_type_case("int"), Some(true));
        assert_eq!(is_signed_type_case("unsigned long long"), Some(false));
        assert_eq!(is_signed_type_case("enum signed_enum"), Some(true));
        assert_eq!(is_signed_type_case("missing"), None);
    }
}
