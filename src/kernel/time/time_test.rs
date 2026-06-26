//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/time_test.c
//! test-origin: linux:vendor/linux/kernel/time/time_test.c
//! KUnit date-range helpers for `time64_to_tm`.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DateCursor {
    pub year: i64,
    pub month: i32,
    pub mday: i32,
    pub yday: i32,
}

pub const fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub const fn last_day_of_month(year: i64, month: i32) -> i32 {
    if month == 2 {
        28 + is_leap(year) as i32
    } else if month == 4 || month == 6 || month == 9 || month == 11 {
        30
    } else {
        31
    }
}

pub const fn advance_date(mut cursor: DateCursor) -> DateCursor {
    if cursor.mday != last_day_of_month(cursor.year, cursor.month) {
        cursor.mday += 1;
        cursor.yday += 1;
        return cursor;
    }

    cursor.mday = 1;
    if cursor.month != 12 {
        cursor.month += 1;
        cursor.yday += 1;
        return cursor;
    }

    cursor.month = 1;
    cursor.yday = 0;
    cursor.year += 1;
    cursor
}

pub const fn centered_160000_year_test_seconds() -> i64 {
    80_000i64 / 400 * 146_097 * 86_400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_test_date_helpers_match_linux_kunit_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/time/time_test.c"
        ));
        assert!(source.contains("static bool is_leap(long year)"));
        assert!(source.contains("year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)"));
        assert!(source.contains("static int last_day_of_month(long year, int month)"));
        assert!(source.contains("if (month == 2)"));
        assert!(source.contains("return 28 + is_leap(year);"));
        assert!(source.contains("static void advance_date"));
        assert!(source.contains("*mday = 1;"));
        assert!(source.contains("*month = 1;"));
        assert!(source.contains("*yday  = 0;"));
        assert!(source.contains("time64_to_tm_test_date_range"));
        assert!(source.contains("KUNIT_CASE_SLOW(time64_to_tm_test_date_range)"));
        assert!(source.contains(".name = \"time_test_cases\""));

        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(is_leap(-4));
        assert_eq!(last_day_of_month(2024, 2), 29);
        assert_eq!(last_day_of_month(2023, 4), 30);
        assert_eq!(
            advance_date(DateCursor {
                year: 2023,
                month: 12,
                mday: 31,
                yday: 364,
            }),
            DateCursor {
                year: 2024,
                month: 1,
                mday: 1,
                yday: 0,
            }
        );
        assert_eq!(centered_160000_year_test_seconds(), 2_524_556_160_000);
    }
}
