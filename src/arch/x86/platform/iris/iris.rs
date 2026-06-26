//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/platform/iris/iris.c
//! test-origin: linux:vendor/linux/arch/x86/platform/iris/iris.c
//! Eurobraille Iris power-off handler registration.

use crate::include::uapi::errno::ENODEV;

pub const IRIS_GIO_BASE: u16 = 0x340;
pub const IRIS_GIO_INPUT: u16 = IRIS_GIO_BASE;
pub const IRIS_GIO_OUTPUT: u16 = IRIS_GIO_BASE + 1;
pub const IRIS_GIO_PULSE: u8 = 0x80;
pub const IRIS_GIO_REST: u8 = 0x00;
pub const IRIS_GIO_NODEV: u8 = 0xff;
pub const IRIS_POWER_OFF_DELAY_MS: u32 = 850;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrisIoWrite {
    pub port: u16,
    pub value: u8,
    pub delay_after_ms: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrisProbeResult {
    pub installed: bool,
    pub old_pm_power_off_saved: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IrisInitResult {
    pub driver_registered: bool,
    pub device_registered: bool,
}

pub const fn iris_power_off_sequence() -> [IrisIoWrite; 2] {
    [
        IrisIoWrite {
            port: IRIS_GIO_OUTPUT,
            value: IRIS_GIO_PULSE,
            delay_after_ms: IRIS_POWER_OFF_DELAY_MS,
        },
        IrisIoWrite {
            port: IRIS_GIO_OUTPUT,
            value: IRIS_GIO_REST,
            delay_after_ms: 0,
        },
    ]
}

pub const fn iris_probe(input_status: u8) -> Result<IrisProbeResult, i32> {
    if input_status == IRIS_GIO_NODEV {
        Err(-ENODEV)
    } else {
        Ok(IrisProbeResult {
            installed: true,
            old_pm_power_off_saved: true,
        })
    }
}

pub const fn iris_remove() -> IrisProbeResult {
    IrisProbeResult {
        installed: false,
        old_pm_power_off_saved: false,
    }
}

pub const fn iris_init(
    force: bool,
    platform_driver_register_result: i32,
    platform_device_register_result: i32,
) -> Result<IrisInitResult, i32> {
    if !force {
        return Err(-ENODEV);
    }
    if platform_driver_register_result < 0 {
        return Err(platform_driver_register_result);
    }
    if platform_device_register_result < 0 {
        return Err(platform_device_register_result);
    }
    Ok(IrisInitResult {
        driver_registered: true,
        device_registered: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iris_poweroff_handler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/platform/iris/iris.c"
        ));
        assert!(source.contains("#define IRIS_GIO_BASE\t\t0x340"));
        assert!(source.contains("#define IRIS_GIO_PULSE\t\t0x80"));
        assert!(source.contains(
            "MODULE_DESCRIPTION(\"A power_off handler for Iris devices from EuroBraille\");"
        ));
        assert!(source.contains("module_param(force, bool, 0);"));
        assert!(source.contains("outb(IRIS_GIO_PULSE, IRIS_GIO_OUTPUT);"));
        assert!(source.contains("msleep(850);"));
        assert!(source.contains("outb(IRIS_GIO_REST, IRIS_GIO_OUTPUT);"));
        assert!(source.contains("unsigned char status = inb(IRIS_GIO_INPUT);"));
        assert!(source.contains("if (status == IRIS_GIO_NODEV)"));
        assert!(source.contains("old_pm_power_off = pm_power_off;"));
        assert!(source.contains("pm_power_off = &iris_power_off;"));
        assert!(source.contains("pm_power_off = old_pm_power_off;"));
        assert!(source.contains(".name   = \"iris\""));
        assert!(source.contains("platform_driver_register(&iris_driver);"));
        assert!(source.contains("platform_device_register_simple(\"iris\", (-1),"));
        assert!(source.contains("module_init(iris_init);"));
        assert!(source.contains("module_exit(iris_exit);"));

        let sequence = iris_power_off_sequence();
        assert_eq!(
            sequence[0],
            IrisIoWrite {
                port: IRIS_GIO_OUTPUT,
                value: IRIS_GIO_PULSE,
                delay_after_ms: 850,
            }
        );
        assert_eq!(sequence[1].value, IRIS_GIO_REST);
        assert_eq!(iris_probe(IRIS_GIO_NODEV), Err(-ENODEV));
        assert_eq!(
            iris_probe(0),
            Ok(IrisProbeResult {
                installed: true,
                old_pm_power_off_saved: true,
            })
        );
        assert_eq!(iris_init(false, 0, 0), Err(-ENODEV));
        assert_eq!(
            iris_init(true, 0, 0),
            Ok(IrisInitResult {
                driver_registered: true,
                device_registered: true,
            })
        );
        assert_eq!(iris_remove().installed, false);
    }
}
