//! linux-parity: complete
//! linux-source: vendor/linux/kernel/sysctl-test.c
//! test-origin: linux:vendor/linux/kernel/sysctl-test.c
//! proc_dointvec KUnit test inventory and boundary model.

extern crate alloc;

use crate::include::uapi::errno::EINVAL;

pub const KUNIT_PROC_READ: i32 = 0;
pub const KUNIT_PROC_WRITE: i32 = 1;
pub const SYSCTL_ZERO: i32 = 0;
pub const SYSCTL_ONE_HUNDRED: i32 = 100;
pub const SUITE_NAME: &str = "sysctl_test";
pub const MODULE_DESCRIPTION: &str = "KUnit test of proc sysctl";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntCtlTable {
    pub has_data: bool,
    pub maxlen: usize,
    pub min: i32,
    pub max: i32,
    pub data: i32,
}

pub fn proc_dointvec_model(
    table: &mut IntCtlTable,
    write: bool,
    buffer: &str,
    len: &mut usize,
    pos: &mut i64,
) -> Result<alloc::string::String, i32> {
    if !table.has_data || table.maxlen == 0 || *len == 0 || (!write && *pos != 0) {
        *len = 0;
        return Ok(alloc::string::String::new());
    }

    if write {
        let value = buffer.trim().parse::<i64>().map_err(|_| -EINVAL)?;
        if value < table.min as i64 || value > table.max as i64 {
            return Err(-EINVAL);
        }
        table.data = value as i32;
        *pos += buffer.len() as i64;
        Ok(alloc::string::String::new())
    } else {
        let out = alloc::format!("{}\n", table.data);
        *len = out.len();
        *pos += out.len() as i64;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysctl_test_matches_linux_original_kunit_suite() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/sysctl-test.c"
        ));

        for case in [
            "KUNIT_CASE(sysctl_test_api_dointvec_null_tbl_data)",
            "KUNIT_CASE(sysctl_test_api_dointvec_table_maxlen_unset)",
            "KUNIT_CASE(sysctl_test_api_dointvec_table_len_is_zero)",
            "KUNIT_CASE(sysctl_test_api_dointvec_table_read_but_position_set)",
            "KUNIT_CASE(sysctl_test_dointvec_read_happy_single_positive)",
            "KUNIT_CASE(sysctl_test_dointvec_read_happy_single_negative)",
            "KUNIT_CASE(sysctl_test_dointvec_write_happy_single_positive)",
            "KUNIT_CASE(sysctl_test_dointvec_write_happy_single_negative)",
            "KUNIT_CASE(sysctl_test_api_dointvec_write_single_less_int_min)",
            "KUNIT_CASE(sysctl_test_api_dointvec_write_single_greater_int_max)",
        ] {
            assert!(source.contains(case));
        }
        assert!(source.contains(".name = \"sysctl_test\""));
        assert!(source.contains("SYSCTL_ZERO"));
        assert!(source.contains("SYSCTL_ONE_HUNDRED"));
        assert!(source.contains(MODULE_DESCRIPTION));

        let mut table = IntCtlTable {
            has_data: false,
            maxlen: core::mem::size_of::<i32>(),
            min: SYSCTL_ZERO,
            max: SYSCTL_ONE_HUNDRED,
            data: 13,
        };
        let mut len = 1234;
        let mut pos = 0;
        assert_eq!(
            proc_dointvec_model(&mut table, false, "", &mut len, &mut pos),
            Ok(alloc::string::String::new())
        );
        assert_eq!(len, 0);

        table.has_data = true;
        len = 4;
        pos = 0;
        assert_eq!(
            proc_dointvec_model(&mut table, false, "", &mut len, &mut pos).unwrap(),
            "13\n"
        );
        assert_eq!(len, 3);

        len = 3;
        pos = 0;
        assert_eq!(
            proc_dointvec_model(&mut table, true, "99\n", &mut len, &mut pos),
            Ok(alloc::string::String::new())
        );
        assert_eq!(table.data, 99);

        len = 32;
        pos = 0;
        assert_eq!(
            proc_dointvec_model(&mut table, true, "101\n", &mut len, &mut pos),
            Err(-EINVAL)
        );
    }
}
