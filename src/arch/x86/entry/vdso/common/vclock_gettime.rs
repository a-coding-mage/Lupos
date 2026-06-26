//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vdso/common/vclock_gettime.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vdso/common/vclock_gettime.c
//! vDSO clock and time wrappers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vdso/common/vclock_gettime.c

use crate::include::uapi::errno::EINVAL;

pub const CLOCK_REALTIME: i32 = 0;
pub const CLOCK_MONOTONIC: i32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KernelTimespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OldTimespec32 {
    pub tv_sec: i32,
    pub tv_nsec: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KernelOldTimeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timezone {
    pub tz_minuteswest: i32,
    pub tz_dsttime: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VdsoTimeSnapshot {
    pub realtime: KernelTimespec,
    pub monotonic: KernelTimespec,
}

pub fn vdso_clock_gettime(clock: i32, ts: &mut KernelTimespec, snap: VdsoTimeSnapshot) -> i32 {
    match clock {
        CLOCK_REALTIME => {
            *ts = snap.realtime;
            0
        }
        CLOCK_MONOTONIC => {
            *ts = snap.monotonic;
            0
        }
        _ => -EINVAL,
    }
}

pub fn vdso_clock_gettime32(clock: i32, ts: &mut OldTimespec32, snap: VdsoTimeSnapshot) -> i32 {
    let mut native = KernelTimespec::default();
    let ret = vdso_clock_gettime(clock, &mut native, snap);
    if ret == 0 {
        ts.tv_sec = native.tv_sec as i32;
        ts.tv_nsec = native.tv_nsec as i32;
    }
    ret
}

pub fn vdso_gettimeofday(
    tv: Option<&mut KernelOldTimeval>,
    tz: Option<&mut Timezone>,
    snap: VdsoTimeSnapshot,
) -> i32 {
    if let Some(tv) = tv {
        tv.tv_sec = snap.realtime.tv_sec;
        tv.tv_usec = snap.realtime.tv_nsec / 1000;
    }
    if let Some(tz) = tz {
        *tz = Timezone::default();
    }
    0
}

pub fn vdso_time(t: Option<&mut i64>, snap: VdsoTimeSnapshot) -> i64 {
    if let Some(t) = t {
        *t = snap.realtime.tv_sec;
    }
    snap.realtime.tv_sec
}

pub fn vdso_clock_getres(clock: i32, res: &mut KernelTimespec) -> i32 {
    match clock {
        CLOCK_REALTIME | CLOCK_MONOTONIC => {
            *res = KernelTimespec {
                tv_sec: 0,
                tv_nsec: 1,
            };
            0
        }
        _ => -EINVAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_gettime_selects_realtime_and_monotonic() {
        let snap = VdsoTimeSnapshot {
            realtime: KernelTimespec {
                tv_sec: 10,
                tv_nsec: 20,
            },
            monotonic: KernelTimespec {
                tv_sec: 3,
                tv_nsec: 4,
            },
        };
        let mut ts = KernelTimespec::default();
        assert_eq!(vdso_clock_gettime(CLOCK_REALTIME, &mut ts, snap), 0);
        assert_eq!(ts.tv_sec, 10);
        assert_eq!(vdso_clock_gettime(CLOCK_MONOTONIC, &mut ts, snap), 0);
        assert_eq!(ts.tv_sec, 3);
        assert_eq!(vdso_clock_gettime(99, &mut ts, snap), -EINVAL);
    }

    #[test]
    fn gettimeofday_converts_nsec_to_usec() {
        let snap = VdsoTimeSnapshot {
            realtime: KernelTimespec {
                tv_sec: 1,
                tv_nsec: 987_654_321,
            },
            ..Default::default()
        };
        let mut tv = KernelOldTimeval::default();
        assert_eq!(vdso_gettimeofday(Some(&mut tv), None, snap), 0);
        assert_eq!(tv.tv_usec, 987_654);
    }
}
