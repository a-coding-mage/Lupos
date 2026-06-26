//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/smb2maperror_test.c
//! test-origin: linux:vendor/linux/fs/smb/client/smb2maperror_test.c
//! SMB2 map-error KUnit suite metadata.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusToPosixError {
    pub smb2_status: u32,
    pub posix_error: i32,
    pub status_string: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Smb2MaperrorSuite {
    pub name: &'static str,
    pub cases: &'static [&'static str],
    pub module_license: &'static str,
    pub module_description: &'static str,
}

pub const SMB2_MAPERROR_SUITE: Smb2MaperrorSuite = Smb2MaperrorSuite {
    name: "smb2_maperror",
    cases: &["maperror_test_check_search"],
    module_license: "GPL",
    module_description: "KUnit tests of SMB2 maperror",
};

pub fn test_cmp_map(expect: &StatusToPosixError, result: Option<&StatusToPosixError>) -> bool {
    match result {
        Some(result) => {
            expect.smb2_status == result.smb2_status
                && expect.posix_error == result.posix_error
                && expect.status_string == result.status_string
        }
        None => false,
    }
}

pub fn maperror_test_check_search<'a>(
    table: &'a [StatusToPosixError],
    lookup: impl Fn(u32) -> Option<&'a StatusToPosixError>,
) -> bool {
    table
        .iter()
        .all(|expect| test_cmp_map(expect, lookup(expect.smb2_status)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smb2_maperror_kunit_suite_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/smb2maperror_test.c"
        ));
        assert!(source.contains("#include <kunit/test.h>"));
        assert!(source.contains("#include \"cifsglob.h\""));
        assert!(source.contains("#include \"smb2glob.h\""));
        assert!(source.contains("#include \"smb2proto.h\""));
        assert!(source.contains("test_cmp_map"));
        assert!(source.contains("smb2_get_err_map_test(expect->smb2_status);"));
        assert!(source.contains("KUNIT_ASSERT_NOT_NULL(test, result);"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, expect->smb2_status"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, expect->posix_error"));
        assert!(source.contains("KUNIT_EXPECT_STREQ(test, expect->status_string"));
        assert!(source.contains("for (i = 0; i < smb2_error_map_num; i++)"));
        assert!(source.contains("KUNIT_CASE(maperror_test_check_search)"));
        assert!(source.contains(".name = \"smb2_maperror\""));
        assert!(source.contains("kunit_test_suite(maperror_suite);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert!(source.contains("MODULE_DESCRIPTION(\"KUnit tests of SMB2 maperror\")"));

        let expected = [
            StatusToPosixError {
                smb2_status: 0xc000_000f,
                posix_error: -2,
                status_string: "STATUS_NO_SUCH_FILE",
            },
            StatusToPosixError {
                smb2_status: 0xc000_0034,
                posix_error: -2,
                status_string: "STATUS_OBJECT_NAME_NOT_FOUND",
            },
        ];
        assert_eq!(SMB2_MAPERROR_SUITE.name, "smb2_maperror");
        assert_eq!(SMB2_MAPERROR_SUITE.cases, ["maperror_test_check_search"]);
        assert!(test_cmp_map(&expected[0], Some(&expected[0])));
        assert!(!test_cmp_map(&expected[0], Some(&expected[1])));
        assert!(maperror_test_check_search(&expected, |status| expected
            .iter()
            .find(|entry| entry.smb2_status == status)));
    }
}
