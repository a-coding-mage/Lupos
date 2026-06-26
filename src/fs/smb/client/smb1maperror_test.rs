//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/smb1maperror_test.c
//! test-origin: linux:vendor/linux/fs/smb/client/smb1maperror_test.c
//! SMB1 map-error KUnit suite metadata.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NtStatusToDosErr {
    pub dos_class: u8,
    pub dos_code: u16,
    pub ntstatus: u32,
    pub nt_errstr: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmbToPosixError {
    pub smb_err: u32,
    pub posix_code: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Smb1MaperrorSuite {
    pub name: &'static str,
    pub cases: &'static [&'static str],
    pub module_license: &'static str,
    pub module_description: &'static str,
}

pub const SMB1_MAPERROR_SUITE: Smb1MaperrorSuite = Smb1MaperrorSuite {
    name: "smb1_maperror",
    cases: &[
        "check_search_ntstatus_to_dos_map",
        "check_search_mapping_table_ERRDOS",
        "check_search_mapping_table_ERRSRV",
    ],
    module_license: "GPL",
    module_description: "KUnit tests of SMB1 maperror",
};

pub fn test_cmp_ntstatus_to_dos_err(expect: &NtStatusToDosErr, result: &NtStatusToDosErr) -> bool {
    expect.dos_class == result.dos_class
        && expect.dos_code == result.dos_code
        && expect.ntstatus == result.ntstatus
        && expect.nt_errstr == result.nt_errstr
}

pub const fn test_cmp_smb_to_posix_error(
    expect: &SmbToPosixError,
    result: &SmbToPosixError,
) -> bool {
    expect.smb_err == result.smb_err && expect.posix_code == result.posix_code
}

pub fn check_search_all<T>(table: &[T], mut lookup_matches: impl FnMut(&T) -> bool) -> bool {
    table.iter().all(|entry| lookup_matches(entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smb1_maperror_kunit_suite_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/smb1maperror_test.c"
        ));
        assert!(source.contains("#include <kunit/test.h>"));
        assert!(source.contains("#include \"smb1proto.h\""));
        assert!(source.contains("#include \"nterr.h\""));
        assert!(source.contains("#include \"smberr.h\""));
        assert!(source.contains("DEFINE_CHECK_SEARCH_FUNC"));
        assert!(source.contains("test_cmp_ntstatus_to_dos_err"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, expect->dos_class"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, expect->dos_code"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, expect->ntstatus"));
        assert!(source.contains("KUNIT_EXPECT_STREQ(test, expect->nt_errstr"));
        assert!(source.contains("test_cmp_smb_to_posix_error"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, expect->smb_err"));
        assert!(source.contains("KUNIT_EXPECT_EQ(test, expect->posix_code"));
        assert!(source.contains("KUNIT_CASE(check_search_ntstatus_to_dos_map)"));
        assert!(source.contains("KUNIT_CASE(check_search_mapping_table_ERRDOS)"));
        assert!(source.contains("KUNIT_CASE(check_search_mapping_table_ERRSRV)"));
        assert!(source.contains(".name = \"smb1_maperror\""));
        assert!(source.contains("kunit_test_suite(maperror_suite);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert!(source.contains("MODULE_DESCRIPTION(\"KUnit tests of SMB1 maperror\")"));

        let nt = NtStatusToDosErr {
            dos_class: 1,
            dos_code: 2,
            ntstatus: 0xc000_000f,
            nt_errstr: "STATUS_NO_SUCH_FILE",
        };
        let posix = SmbToPosixError {
            smb_err: 2,
            posix_code: -2,
        };
        assert_eq!(SMB1_MAPERROR_SUITE.name, "smb1_maperror");
        assert_eq!(SMB1_MAPERROR_SUITE.cases.len(), 3);
        assert!(test_cmp_ntstatus_to_dos_err(&nt, &nt));
        assert!(test_cmp_smb_to_posix_error(&posix, &posix));
        assert!(check_search_all(&[posix], |entry| entry.smb_err == 2));
        assert!(!check_search_all(&[posix], |entry| entry.posix_code == -5));
    }
}
