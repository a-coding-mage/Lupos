//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/apm_32.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/apm_32.c
//! 32-bit APM BIOS policy and user-event queues.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/apm_32.c

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

pub const HZ: u32 = 100;
pub const APM_CHECK_TIMEOUT: u32 = HZ;
pub const DEFAULT_BOUNCE_INTERVAL: u32 = 3 * HZ;
pub const APM_MAX_EVENTS: usize = 20;
pub const APM_BIOS_MAGIC: u16 = 0x4101;
pub const DEFAULT_IDLE_THRESHOLD: i32 = 100;
pub const DEFAULT_IDLE_PERIOD: i32 = 100 / 3;
pub const APM_MINOR_DEV: u8 = 134;

pub const APM_16_BIT_SUPPORT: u16 = 1 << 0;
pub const APM_32_BIT_SUPPORT: u16 = 1 << 1;
pub const APM_IDLE_SLOWS_CLOCK: u16 = 1 << 2;
pub const APM_BIOS_DISABLED: u16 = 1 << 3;
pub const APM_BIOS_DISENGAGED: u16 = 1 << 4;

pub const APM_SYS_STANDBY: u16 = 0x0001;
pub const APM_SYS_SUSPEND: u16 = 0x0002;
pub const APM_NORMAL_RESUME: u16 = 0x0003;
pub const APM_CRITICAL_RESUME: u16 = 0x0004;
pub const APM_LOW_BATTERY: u16 = 0x0005;
pub const APM_POWER_STATUS_CHANGE: u16 = 0x0006;
pub const APM_UPDATE_TIME: u16 = 0x0007;
pub const APM_CRITICAL_SUSPEND: u16 = 0x0008;
pub const APM_USER_STANDBY: u16 = 0x0009;
pub const APM_USER_SUSPEND: u16 = 0x000a;
pub const APM_STANDBY_RESUME: u16 = 0x000b;
pub const APM_CAPABILITY_CHANGE: u16 = 0x000c;

pub const APM_DISABLED: u8 = 0x01;
pub const APM_CONNECTED: u8 = 0x02;
pub const APM_NOT_CONNECTED: u8 = 0x03;
pub const APM_16_CONNECTED: u8 = 0x05;
pub const APM_32_CONNECTED: u8 = 0x07;
pub const APM_32_UNSUPPORTED: u8 = 0x08;
pub const APM_BAD_DEVICE: u8 = 0x09;
pub const APM_BAD_PARAM: u8 = 0x0a;
pub const APM_NOT_ENGAGED: u8 = 0x0b;
pub const APM_BAD_FUNCTION: u8 = 0x0c;
pub const APM_RESUME_DISABLED: u8 = 0x0d;
pub const APM_NO_ERROR: u8 = 0x53;
pub const APM_BAD_STATE: u8 = 0x60;
pub const APM_NO_EVENTS: u8 = 0x80;
pub const APM_NOT_PRESENT: u8 = 0x86;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApmInfo {
    pub disabled: bool,
    pub debug: bool,
    pub smp: bool,
    pub power_off: bool,
    pub allow_ints: bool,
    pub realmode_power_off: bool,
    pub broken_psr: bool,
    pub idle_threshold: i32,
    pub idle_period: i32,
    pub bounce_interval: i32,
}

impl Default for ApmInfo {
    fn default() -> Self {
        Self {
            disabled: false,
            debug: false,
            smp: false,
            power_off: false,
            allow_ints: false,
            realmode_power_off: false,
            broken_psr: false,
            idle_threshold: DEFAULT_IDLE_THRESHOLD,
            idle_period: DEFAULT_IDLE_PERIOD,
            bounce_interval: DEFAULT_BOUNCE_INTERVAL as i32,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApmUser {
    event_head: usize,
    event_tail: usize,
    events: [u16; APM_MAX_EVENTS],
    pub overflowed: bool,
}

impl Default for ApmUser {
    fn default() -> Self {
        Self {
            event_head: 0,
            event_tail: 0,
            events: [0; APM_MAX_EVENTS],
            overflowed: false,
        }
    }
}

impl ApmUser {
    pub const fn queue_empty(&self) -> bool {
        self.event_head == self.event_tail
    }

    pub fn queue_event(&mut self, event: u16) {
        self.event_head += 1;
        if self.event_head >= APM_MAX_EVENTS {
            self.event_head = 0;
        }
        if self.event_head == self.event_tail {
            self.event_tail += 1;
            if self.event_tail >= APM_MAX_EVENTS {
                self.event_tail = 0;
            }
            self.overflowed = true;
        }
        self.events[self.event_head] = event;
    }

    pub fn pop_event(&mut self) -> Option<u16> {
        if self.queue_empty() {
            return None;
        }
        self.event_tail += 1;
        if self.event_tail >= APM_MAX_EVENTS {
            self.event_tail = 0;
        }
        Some(self.events[self.event_tail])
    }
}

pub const fn apm_event_name(event: u16) -> Option<&'static str> {
    match event {
        APM_SYS_STANDBY => Some("system standby"),
        APM_SYS_SUSPEND => Some("system suspend"),
        APM_NORMAL_RESUME => Some("normal resume"),
        APM_CRITICAL_RESUME => Some("critical resume"),
        APM_LOW_BATTERY => Some("low battery"),
        APM_POWER_STATUS_CHANGE => Some("power status change"),
        APM_UPDATE_TIME => Some("update time"),
        APM_CRITICAL_SUSPEND => Some("critical suspend"),
        APM_USER_STANDBY => Some("user standby"),
        APM_USER_SUSPEND => Some("user suspend"),
        APM_STANDBY_RESUME => Some("system standby resume"),
        APM_CAPABILITY_CHANGE => Some("capabilities change"),
        _ => None,
    }
}

pub const fn apm_error_message(code: u8) -> Option<&'static str> {
    match code {
        APM_DISABLED => Some("Power management disabled"),
        APM_CONNECTED => Some("Real mode interface already connected"),
        APM_NOT_CONNECTED => Some("Interface not connected"),
        APM_16_CONNECTED => Some("16 bit interface already connected"),
        APM_32_CONNECTED => Some("32 bit interface already connected"),
        APM_32_UNSUPPORTED => Some("32 bit interface not supported"),
        APM_BAD_DEVICE => Some("Unrecognized device ID"),
        APM_BAD_PARAM => Some("Parameter out of range"),
        APM_NOT_ENGAGED => Some("Interface not engaged"),
        APM_BAD_FUNCTION => Some("Function not supported"),
        APM_RESUME_DISABLED => Some("Resume timer disabled"),
        APM_BAD_STATE => Some("Unable to enter requested state"),
        APM_NO_ERROR => Some("BIOS did not set a return code"),
        APM_NOT_PRESENT => Some("No APM present"),
        _ => None,
    }
}

pub fn apm_setup(info: &mut ApmInfo, value: &str) {
    for raw in value.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }

        if token.starts_with("off") {
            info.disabled = true;
        }
        if token.starts_with("on") {
            info.disabled = false;
        }
        if let Some(v) = option_value(token, "bounce-interval=", "bounce_interval=") {
            info.bounce_interval = parse_int(v);
        }
        if let Some(v) = option_value(token, "idle-threshold=", "idle_threshold=") {
            info.idle_threshold = parse_int(v);
        }
        if let Some(v) = option_value(token, "idle-period=", "idle_period=") {
            info.idle_period = parse_int(v);
        }

        let (invert, body) = if token.starts_with("no-") || token.starts_with("no_") {
            (true, &token[3..])
        } else {
            (false, token)
        };
        let enabled = !invert;
        if body.starts_with("debug") {
            info.debug = enabled;
        }
        if body.starts_with("power-off") || body.starts_with("power_off") {
            info.power_off = enabled;
        }
        if body.starts_with("smp") {
            info.smp = enabled;
            info.idle_threshold = 100;
        }
        if body.starts_with("allow-ints") || body.starts_with("allow_ints") {
            info.allow_ints = enabled;
        }
        if body.starts_with("broken-psr") || body.starts_with("broken_psr") {
            info.broken_psr = enabled;
        }
        if body.starts_with("realmode-power-off") || body.starts_with("realmode_power_off") {
            info.realmode_power_off = enabled;
        }
    }
}

pub const fn validate_apm_32bit_support(flags: u16) -> bool {
    (flags & APM_32_BIT_SUPPORT) != 0
}

pub const fn apm_init_supported(
    running_32bit_kernel: bool,
    bios_version: u16,
    bios_flags: u16,
    disabled: bool,
) -> Result<(), i32> {
    if !running_32bit_kernel {
        return Err(EOPNOTSUPP);
    }
    if disabled || bios_version == 0 || !validate_apm_32bit_support(bios_flags) {
        return Err(ENODEV);
    }
    Ok(())
}

fn option_value<'a>(token: &'a str, dash: &str, under: &str) -> Option<&'a str> {
    token
        .strip_prefix(dash)
        .or_else(|| token.strip_prefix(under))
}

fn parse_int(value: &str) -> i32 {
    if let Some(hex) = value.strip_prefix("0x") {
        i32::from_str_radix(hex, 16).unwrap_or(0)
    } else {
        value.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_and_error_tables_match_linux_strings() {
        assert_eq!(apm_event_name(APM_SYS_STANDBY), Some("system standby"));
        assert_eq!(
            apm_event_name(APM_CAPABILITY_CHANGE),
            Some("capabilities change")
        );
        assert_eq!(
            apm_error_message(APM_BAD_FUNCTION),
            Some("Function not supported")
        );
    }

    #[test]
    fn user_event_queue_uses_linux_ring_layout_and_overflow_drop() {
        let mut user = ApmUser::default();
        for event in 1..=APM_MAX_EVENTS as u16 {
            user.queue_event(event);
        }
        assert!(user.overflowed);
        assert_eq!(user.pop_event(), Some(2));
        assert_eq!(user.pop_event(), Some(3));
    }

    #[test]
    fn setup_parser_handles_inverted_and_underscore_options() {
        let mut info = ApmInfo::default();
        apm_setup(
            &mut info,
            "debug,no-power-off,allow_ints,broken-psr,realmode_power_off,idle-period=12,smp",
        );
        assert!(info.debug);
        assert!(!info.power_off);
        assert!(info.allow_ints);
        assert!(info.broken_psr);
        assert!(info.realmode_power_off);
        assert!(info.smp);
        assert_eq!(info.idle_threshold, 100);
        assert_eq!(info.idle_period, 12);
    }

    #[test]
    fn init_support_requires_32bit_kernel_and_bios_flag() {
        assert_eq!(
            apm_init_supported(false, 0x102, APM_32_BIT_SUPPORT, false),
            Err(EOPNOTSUPP)
        );
        assert_eq!(apm_init_supported(true, 0x102, 0, false), Err(ENODEV));
        assert_eq!(
            apm_init_supported(true, 0x102, APM_32_BIT_SUPPORT, false),
            Ok(())
        );
    }
}
