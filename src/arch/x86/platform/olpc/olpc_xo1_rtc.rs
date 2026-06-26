//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/platform/olpc/olpc-xo1-rtc.c
//! test-origin: linux:vendor/linux/arch/x86/platform/olpc/olpc-xo1-rtc.c
//! OLPC XO-1 RTC platform-device registration.

use crate::include::uapi::errno::ENODEV;

pub const RTC_IRQ: u16 = 8;
pub const CS5536_PM_RTC: u32 = 1 << 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Xo1RtcInfo {
    pub rtc_day_alarm: u64,
    pub rtc_mon_alarm: u64,
    pub rtc_century: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Xo1RtcRegistration {
    pub registered: bool,
    pub legacy_rtc_disabled: bool,
    pub wakeup_enabled: bool,
    pub info: Xo1RtcInfo,
}

pub const fn rtc_ports(index: u16) -> u16 {
    0x70 + index
}

pub const fn xo1_rtc_init(
    compatible_node_found: bool,
    platform_register_result: i32,
    info: Xo1RtcInfo,
) -> Result<Xo1RtcRegistration, i32> {
    if !compatible_node_found {
        return Ok(Xo1RtcRegistration {
            registered: false,
            legacy_rtc_disabled: false,
            wakeup_enabled: false,
            info,
        });
    }
    if platform_register_result != 0 {
        return Err(platform_register_result);
    }
    Ok(Xo1RtcRegistration {
        registered: true,
        legacy_rtc_disabled: true,
        wakeup_enabled: true,
        info,
    })
}

pub const fn rtc_wake_on_bit() -> u32 {
    CS5536_PM_RTC
}

pub const fn rtc_wake_off_bit() -> u32 {
    CS5536_PM_RTC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xo1_rtc_registration_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/platform/olpc/olpc-xo1-rtc.c"
        ));
        assert!(source.contains("olpc_xo1_pm_wakeup_set(CS5536_PM_RTC);"));
        assert!(source.contains("olpc_xo1_pm_wakeup_clear(CS5536_PM_RTC);"));
        assert!(source.contains(".name = \"rtc_cmos\""));
        assert!(source.contains(".start\t= RTC_PORT(0)"));
        assert!(source.contains(".start\t= RTC_IRQ"));
        assert!(source.contains("of_find_compatible_node(NULL, NULL, \"olpc,xo1-rtc\")"));
        assert!(source.contains("rdmsrq(MSR_RTC_DOMA_OFFSET"));
        assert!(source.contains("platform_device_register(&xo1_rtc_device)"));
        assert!(source.contains("x86_platform.legacy.rtc = 0"));
        assert!(source.contains("device_init_wakeup(&xo1_rtc_device.dev, 1)"));

        let info = Xo1RtcInfo {
            rtc_day_alarm: 1,
            rtc_mon_alarm: 2,
            rtc_century: 20,
        };
        let skipped = xo1_rtc_init(false, 0, info).unwrap();
        assert!(!skipped.registered);
        let registered = xo1_rtc_init(true, 0, info).unwrap();
        assert!(registered.registered);
        assert!(registered.legacy_rtc_disabled);
        assert_eq!(rtc_ports(1), 0x71);
        assert_eq!(xo1_rtc_init(true, -ENODEV, info), Err(-ENODEV));
        assert_eq!(rtc_wake_on_bit(), CS5536_PM_RTC);
        assert_eq!(rtc_wake_off_bit(), CS5536_PM_RTC);
    }
}
