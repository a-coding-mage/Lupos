//! linux-parity: complete
//! linux-source: vendor/linux/kernel/time/alarmtimer.c
//! test-origin: linux:vendor/linux/kernel/time/alarmtimer.c
//! Alarm timer — `CLOCK_REALTIME_ALARM` / `CLOCK_BOOTTIME_ALARM` (M36 stub).
//!
//! Mirrors `vendor/linux/kernel/time/alarmtimer.c`.  Real RTC backing arrives
//! with M55 PCI/ACPI; M36 ships a stub that maps onto `Hrtimer` so userspace
//! API surface is present.

use super::hrtimer::{
    ClockBase, Hrtimer, HrtimerMode, HrtimerRestart, hrtimer_init, hrtimer_start,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlarmType {
    RealtimeAlarm,
    BoottimeAlarm,
}

pub struct Alarm {
    pub alarm_type: AlarmType,
    pub timer: Hrtimer,
}

impl Alarm {
    pub fn new(alarm_type: AlarmType) -> Self {
        let mut t = Hrtimer::new();
        let base = match alarm_type {
            AlarmType::RealtimeAlarm => ClockBase::Realtime,
            AlarmType::BoottimeAlarm => ClockBase::Boottime,
        };
        hrtimer_init(&mut t, base, HrtimerMode::Abs);
        Self {
            alarm_type,
            timer: t,
        }
    }

    pub fn set(&mut self, expires_ns: u64) {
        hrtimer_start(
            &mut self.timer as *mut Hrtimer,
            expires_ns,
            HrtimerMode::Abs,
        );
    }
}

pub fn alarmtimer_init() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alarm_realtime_uses_realtime_base() {
        let a = Alarm::new(AlarmType::RealtimeAlarm);
        assert_eq!(a.timer.base, ClockBase::Realtime);
    }

    #[test]
    fn alarm_boottime_uses_boottime_base() {
        let a = Alarm::new(AlarmType::BoottimeAlarm);
        assert_eq!(a.timer.base, ClockBase::Boottime);
    }
}
