//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/rtc.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/rtc.c
//! CMOS real-time clock helpers.
//!
//! References:
//! - vendor/linux/arch/x86/kernel/rtc.c

use crate::arch::x86::include::asm::io::{inb, outb};

pub const CMOS_INDEX_PORT: u16 = 0x70;
pub const CMOS_DATA_PORT: u16 = 0x71;
pub const CMOS_NMI_DISABLE: u8 = 1 << 7;

pub const RTC_SECONDS: u8 = 0x00;
pub const RTC_MINUTES: u8 = 0x02;
pub const RTC_HOURS: u8 = 0x04;
pub const RTC_DAY_OF_MONTH: u8 = 0x07;
pub const RTC_MONTH: u8 = 0x08;
pub const RTC_YEAR: u8 = 0x09;
pub const RTC_REG_A: u8 = 0x0A;
pub const RTC_REG_B: u8 = 0x0B;

pub const RTC_UIP: u8 = 1 << 7;
pub const RTC_DM_BINARY: u8 = 1 << 2;

const RTC_UIP_MAX_POLLS: usize = 1024;

pub trait CmosReader {
    fn read(&mut self, reg: u8) -> u8;
}

struct PortCmosReader;

impl CmosReader for PortCmosReader {
    fn read(&mut self, reg: u8) -> u8 {
        unsafe { read_cmos(reg) }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RtcDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RtcSnapshot {
    second: u8,
    minute: u8,
    hour: u8,
    day: u8,
    month: u8,
    year: u8,
    control: u8,
}

pub fn bcd_to_binary(value: u8) -> u8 {
    (value & 0x0f) + ((value >> 4) * 10)
}

pub fn mktime64(year0: u32, mon0: u32, day: u32, hour: u32, min: u32, sec: u32) -> i64 {
    let mut mon = mon0 as i64;
    let mut year = year0 as i64;

    mon -= 2;
    if mon <= 0 {
        mon += 12;
        year -= 1;
    }

    (((((year / 4 - year / 100 + year / 400 + 367 * mon / 12 + day as i64) + year * 365
        - 719_499)
        * 24
        + hour as i64)
        * 60
        + min as i64)
        * 60)
        + sec as i64
}

fn read_stable_snapshot<R: CmosReader>(reader: &mut R, max_polls: usize) -> Option<RtcSnapshot> {
    for _ in 0..max_polls {
        if reader.read(RTC_REG_A) & RTC_UIP != 0 {
            continue;
        }

        let snapshot = RtcSnapshot {
            second: reader.read(RTC_SECONDS),
            minute: reader.read(RTC_MINUTES),
            hour: reader.read(RTC_HOURS),
            day: reader.read(RTC_DAY_OF_MONTH),
            month: reader.read(RTC_MONTH),
            year: reader.read(RTC_YEAR),
            control: reader.read(RTC_REG_B),
        };

        if reader.read(RTC_REG_A) & RTC_UIP == 0 {
            return Some(snapshot);
        }
    }
    None
}

fn decode_snapshot(snapshot: RtcSnapshot) -> Option<RtcDateTime> {
    let binary = snapshot.control & RTC_DM_BINARY != 0;
    let decode = |value| {
        if binary { value } else { bcd_to_binary(value) }
    };

    let second = decode(snapshot.second);
    let minute = decode(snapshot.minute);
    let hour = decode(snapshot.hour);
    let day = decode(snapshot.day);
    let month = decode(snapshot.month);
    let year = decode(snapshot.year);

    if second > 60 || minute > 59 || hour > 23 || day == 0 || day > 31 || month == 0 || month > 12 {
        return None;
    }

    let full_year = if year <= 69 {
        2000 + year as u16
    } else {
        1900 + year as u16
    };

    Some(RtcDateTime {
        year: full_year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

pub fn read_persistent_clock_seconds_with<R: CmosReader>(
    reader: &mut R,
    max_polls: usize,
) -> Option<u64> {
    let snapshot = read_stable_snapshot(reader, max_polls)?;
    let dt = decode_snapshot(snapshot)?;
    let seconds = mktime64(
        dt.year as u32,
        dt.month as u32,
        dt.day as u32,
        dt.hour as u32,
        dt.minute as u32,
        dt.second as u32,
    );
    (seconds >= 0).then_some(seconds as u64)
}

pub unsafe fn read_persistent_clock_seconds() -> Option<u64> {
    let mut reader = PortCmosReader;
    read_persistent_clock_seconds_with(&mut reader, RTC_UIP_MAX_POLLS)
}

pub unsafe fn read_cmos(reg: u8) -> u8 {
    unsafe {
        outb(CMOS_INDEX_PORT, CMOS_NMI_DISABLE | reg);
        inb(CMOS_DATA_PORT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtc_ports_and_registers_match_pc_cmos() {
        assert_eq!(CMOS_INDEX_PORT, 0x70);
        assert_eq!(CMOS_DATA_PORT, 0x71);
        assert_eq!(RTC_REG_A, 0x0A);
        assert_eq!(RTC_REG_B, 0x0B);
    }

    #[test]
    fn bcd_conversion_matches_rtc_encoding() {
        assert_eq!(bcd_to_binary(0x00), 0);
        assert_eq!(bcd_to_binary(0x09), 9);
        assert_eq!(bcd_to_binary(0x42), 42);
        assert_eq!(bcd_to_binary(0x59), 59);
    }

    struct FakeCmos {
        regs: [u8; 128],
        uip_reads: usize,
    }

    impl FakeCmos {
        fn new(regs: &[(u8, u8)], uip_reads: usize) -> Self {
            let mut fake = Self {
                regs: [0; 128],
                uip_reads,
            };
            for (reg, value) in regs {
                fake.regs[*reg as usize] = *value;
            }
            fake
        }
    }

    impl CmosReader for FakeCmos {
        fn read(&mut self, reg: u8) -> u8 {
            if reg == RTC_REG_A && self.uip_reads > 0 {
                self.uip_reads -= 1;
                return RTC_UIP;
            }
            self.regs[reg as usize]
        }
    }

    #[test]
    fn mc146818_bcd_snapshot_converts_to_unix_time() {
        let mut cmos = FakeCmos::new(
            &[
                (RTC_SECONDS, 0x56),
                (RTC_MINUTES, 0x34),
                (RTC_HOURS, 0x12),
                (RTC_DAY_OF_MONTH, 0x19),
                (RTC_MONTH, 0x05),
                (RTC_YEAR, 0x26),
                (RTC_REG_B, 0),
            ],
            1,
        );

        let clock = read_persistent_clock_seconds_with(&mut cmos, 8).unwrap();
        assert_eq!(clock, 1_779_194_096);
    }

    #[test]
    fn mc146818_binary_mode_uses_raw_register_values() {
        let mut cmos = FakeCmos::new(
            &[
                (RTC_SECONDS, 56),
                (RTC_MINUTES, 34),
                (RTC_HOURS, 12),
                (RTC_DAY_OF_MONTH, 19),
                (RTC_MONTH, 5),
                (RTC_YEAR, 26),
                (RTC_REG_B, RTC_DM_BINARY),
            ],
            0,
        );

        let clock = read_persistent_clock_seconds_with(&mut cmos, 8).unwrap();
        assert_eq!(clock, 1_779_194_096);
    }

    #[test]
    fn mc146818_year_pivot_matches_linux() {
        let mut year_69 = FakeCmos::new(
            &[
                (RTC_SECONDS, 0x00),
                (RTC_MINUTES, 0x00),
                (RTC_HOURS, 0x00),
                (RTC_DAY_OF_MONTH, 0x01),
                (RTC_MONTH, 0x01),
                (RTC_YEAR, 0x69),
                (RTC_REG_B, 0),
            ],
            0,
        );
        let mut year_70 = FakeCmos::new(
            &[
                (RTC_SECONDS, 0x00),
                (RTC_MINUTES, 0x00),
                (RTC_HOURS, 0x00),
                (RTC_DAY_OF_MONTH, 0x01),
                (RTC_MONTH, 0x01),
                (RTC_YEAR, 0x70),
                (RTC_REG_B, 0),
            ],
            0,
        );

        assert_eq!(
            read_persistent_clock_seconds_with(&mut year_69, 8),
            Some(mktime64(2069, 1, 1, 0, 0, 0) as u64)
        );
        assert_eq!(read_persistent_clock_seconds_with(&mut year_70, 8), Some(0));
    }

    #[test]
    fn mc146818_uip_timeout_leaves_clock_unavailable() {
        let mut cmos = FakeCmos::new(&[(RTC_REG_B, 0)], 16);
        assert_eq!(read_persistent_clock_seconds_with(&mut cmos, 4), None);
    }
}
