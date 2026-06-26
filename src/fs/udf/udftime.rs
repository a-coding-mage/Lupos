//! linux-parity: complete
//! linux-source: vendor/linux/fs/udf/udftime.c
//! test-origin: linux:vendor/linux/fs/udf/udftime.c
//! UDF disk timestamp conversion rules.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timespec64 {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UdfTimestamp {
    pub type_and_timezone: u16,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub centiseconds: u8,
    pub hundreds_of_microseconds: u8,
    pub microseconds: u8,
}

pub const UDF_UNSPECIFIED_OFFSET: i16 = -2047;

pub fn udf_disk_stamp_to_time(src: UdfTimestamp) -> Timespec64 {
    let stamp_type = src.type_and_timezone >> 12;
    let mut offset = 0i16;
    if stamp_type == 1 {
        offset = ((src.type_and_timezone << 4) as i16) >> 4;
        if offset == UDF_UNSPECIFIED_OFFSET {
            offset = 0;
        }
    }

    let mut tv_sec = mktime64(
        src.year as i32,
        src.month as u32,
        src.day as u32,
        src.hour as u32,
        src.minute as u32,
        src.second as u32,
    );
    tv_sec -= offset as i64 * 60;

    let tv_nsec =
        if src.centiseconds < 100 && src.hundreds_of_microseconds < 100 && src.microseconds < 100 {
            1000 * (src.centiseconds as i64 * 10000
                + src.hundreds_of_microseconds as i64 * 100
                + src.microseconds as i64)
        } else {
            0
        };

    Timespec64 { tv_sec, tv_nsec }
}

pub fn udf_time_to_disk_stamp(ts: Timespec64, tz_minuteswest: i16) -> UdfTimestamp {
    let offset = -tz_minuteswest;
    let seconds = ts.tv_sec + offset as i64 * 60;
    let tm = time64_to_tm(seconds);
    let micros = ts.tv_nsec / 1000;
    let centiseconds = (ts.tv_nsec / 10_000_000) as u8;
    let hundreds_of_microseconds = ((micros - centiseconds as i64 * 10000) / 100) as u8;
    let microseconds =
        (micros - centiseconds as i64 * 10000 - hundreds_of_microseconds as i64 * 100) as u8;

    UdfTimestamp {
        type_and_timezone: 0x1000 | (offset as u16 & 0x0fff),
        year: (tm.year + 1900) as u16,
        month: (tm.month + 1) as u8,
        day: tm.month_day as u8,
        hour: tm.hour as u8,
        minute: tm.minute as u8,
        second: tm.second as u8,
        centiseconds,
        hundreds_of_microseconds,
        microseconds,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BrokenDownTime {
    year: i32,
    month: u32,
    month_day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

fn mktime64(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> i64 {
    let days = days_from_civil(year, month, day);
    days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64
}

fn time64_to_tm(seconds: i64) -> BrokenDownTime {
    let days = div_floor(seconds, 86_400);
    let rem = seconds - days * 86_400;
    let (year, month, day) = civil_from_days(days);
    BrokenDownTime {
        year: year - 1900,
        month: month - 1,
        month_day: day,
        hour: (rem / 3_600) as u32,
        minute: ((rem % 3_600) / 60) as u32,
        second: (rem % 60) as u32,
    }
}

fn div_floor(n: i64, d: i64) -> i64 {
    let q = n / d;
    let r = n % d;
    if r != 0 && ((r > 0) != (d > 0)) {
        q - 1
    } else {
        q
    }
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = year as i64 - if month <= 2 { 1 } else { 0 };
    let era = div_floor(y, 400);
    let yoe = y - era * 400;
    let m = month as i64;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = div_floor(z, 146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udf_timestamp_conversion_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/udf/udftime.c"
        ));
        assert!(source.contains("#include \"udfdecl.h\""));
        assert!(source.contains("#include <linux/types.h>"));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/time.h>"));
        assert!(source.contains("udf_disk_stamp_to_time"));
        assert!(source.contains("u16 typeAndTimezone = le16_to_cpu(src.typeAndTimezone);"));
        assert!(source.contains("uint8_t type = typeAndTimezone >> 12;"));
        assert!(source.contains("offset = typeAndTimezone << 4;"));
        assert!(source.contains("offset = (offset >> 4);"));
        assert!(source.contains("if (offset == -2047)"));
        assert!(source.contains("dest->tv_sec = mktime64(year, src.month, src.day"));
        assert!(source.contains("dest->tv_sec -= offset * 60;"));
        assert!(source.contains("src.centiseconds < 100 && src.hundredsOfMicroseconds < 100"));
        assert!(source.contains("dest->tv_nsec = 1000 * (src.centiseconds * 10000"));
        assert!(source.contains("udf_time_to_disk_stamp"));
        assert!(source.contains("offset = -sys_tz.tz_minuteswest;"));
        assert!(
            source.contains("dest->typeAndTimezone = cpu_to_le16(0x1000 | (offset & 0x0FFF));")
        );
        assert!(source.contains("time64_to_tm(seconds, 0, &tm);"));
        assert!(source.contains("dest->centiseconds = ts.tv_nsec / 10000000;"));

        let stamp = UdfTimestamp {
            type_and_timezone: 0x1000,
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            centiseconds: 1,
            hundreds_of_microseconds: 2,
            microseconds: 3,
        };
        assert_eq!(
            udf_disk_stamp_to_time(stamp),
            Timespec64 {
                tv_sec: 0,
                tv_nsec: 10_203_000
            }
        );

        let east_one_hour = UdfTimestamp {
            type_and_timezone: 0x103c,
            year: 1970,
            month: 1,
            day: 1,
            hour: 1,
            minute: 0,
            second: 0,
            centiseconds: 0,
            hundreds_of_microseconds: 0,
            microseconds: 0,
        };
        assert_eq!(udf_disk_stamp_to_time(east_one_hour).tv_sec, 0);

        let bogus_subsecond = UdfTimestamp {
            centiseconds: 100,
            ..stamp
        };
        assert_eq!(udf_disk_stamp_to_time(bogus_subsecond).tv_nsec, 0);

        let round = udf_time_to_disk_stamp(
            Timespec64 {
                tv_sec: 3_600,
                tv_nsec: 10_203_000,
            },
            -60,
        );
        assert_eq!(round.type_and_timezone, 0x103c);
        assert_eq!(round.year, 1970);
        assert_eq!(round.hour, 2);
        assert_eq!(round.centiseconds, 1);
        assert_eq!(round.hundreds_of_microseconds, 2);
        assert_eq!(round.microseconds, 3);
    }
}
