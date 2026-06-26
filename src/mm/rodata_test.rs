//! linux-parity: complete
//! linux-source: vendor/linux/mm/rodata_test.c
//! test-origin: linux:vendor/linux/mm/rodata_test.c
//! Runtime checks for read-only kernel rodata.

pub const TEST_VALUE: i32 = 0xC3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RodataTestResult {
    StartDataChanged,
    WriteDidNotFault,
    DataChangedAfterFault,
    StartNotPageAligned,
    EndNotPageAligned,
    Success,
}

pub const fn rodata_test_result(
    start_value: i32,
    write_faulted: bool,
    final_value: i32,
    rodata_start: usize,
    rodata_end: usize,
    page_size: usize,
) -> RodataTestResult {
    if start_value != TEST_VALUE {
        return RodataTestResult::StartDataChanged;
    }
    if !write_faulted {
        return RodataTestResult::WriteDidNotFault;
    }
    if final_value != TEST_VALUE {
        return RodataTestResult::DataChangedAfterFault;
    }
    if rodata_start % page_size != 0 {
        return RodataTestResult::StartNotPageAligned;
    }
    if rodata_end % page_size != 0 {
        return RodataTestResult::EndNotPageAligned;
    }
    RodataTestResult::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rodata_test_order_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/rodata_test.c"
        ));
        assert!(source.contains("#define TEST_VALUE 0xC3"));
        assert!(source.contains("static const int rodata_test_data = TEST_VALUE;"));
        assert!(source.contains("READ_ONCE(rodata_test_data) != TEST_VALUE"));
        assert!(source.contains("copy_to_kernel_nofault"));
        assert!(source.contains("test data was not read only"));
        assert!(source.contains("PAGE_ALIGNED(__start_rodata)"));
        assert!(source.contains("PAGE_ALIGNED(__end_rodata)"));
        assert!(source.contains("all tests were successful"));

        assert_eq!(
            rodata_test_result(TEST_VALUE, true, TEST_VALUE, 0x1000, 0x2000, 4096),
            RodataTestResult::Success
        );
        assert_eq!(
            rodata_test_result(0, true, TEST_VALUE, 0x1000, 0x2000, 4096),
            RodataTestResult::StartDataChanged
        );
        assert_eq!(
            rodata_test_result(TEST_VALUE, false, TEST_VALUE, 0x1000, 0x2000, 4096),
            RodataTestResult::WriteDidNotFault
        );
    }
}
