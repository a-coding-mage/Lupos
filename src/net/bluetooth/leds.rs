//! linux-parity: complete
//! linux-source: vendor/linux/net/bluetooth/leds.c
//! test-origin: linux:vendor/linux/net/bluetooth/leds.c
//! Bluetooth LED trigger helpers.

pub const LED_OFF: u8 = 0;
pub const LED_FULL: u8 = 255;
pub const BT_POWER_LED_TRIGGER: &str = "bluetooth-power";
pub const HCI_UP: u32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedTrigger {
    pub name: &'static str,
    pub brightness: u8,
    pub registered: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HciDev {
    pub name: &'static str,
    pub up: bool,
    pub power_led: Option<LedTrigger>,
}

pub fn hci_leds_update_powered(
    hdev: &mut HciDev,
    enabled: bool,
    any_other_hci_dev_up: bool,
    bt_power: &mut LedTrigger,
) {
    if let Some(mut power_led) = hdev.power_led {
        power_led.brightness = if enabled { LED_FULL } else { LED_OFF };
        hdev.power_led = Some(power_led);
    }

    let global_enabled = if enabled { true } else { any_other_hci_dev_up };
    bt_power.brightness = if global_enabled { LED_FULL } else { LED_OFF };
}

pub fn power_activate(hdev: &HciDev, led_cdev: &mut LedTrigger) -> i32 {
    led_cdev.brightness = if hdev.up { LED_FULL } else { LED_OFF };
    0
}

pub fn led_allocate_basic(
    hdev: &HciDev,
    allocation_ok: bool,
    name_allocation_ok: bool,
    register_ok: bool,
) -> Option<LedTrigger> {
    if !allocation_ok || !name_allocation_ok || !register_ok {
        return None;
    }
    let name = match hdev.name {
        "hci0" => "hci0-power",
        "hci1" => "hci1-power",
        _ => "hci-power",
    };
    Some(LedTrigger {
        name,
        brightness: LED_OFF,
        registered: true,
    })
}

pub fn hci_leds_init(hdev: &mut HciDev, allocation_ok: bool, name_ok: bool, register_ok: bool) {
    hdev.power_led = led_allocate_basic(hdev, allocation_ok, name_ok, register_ok);
}

pub const fn bt_leds_init() -> LedTrigger {
    LedTrigger {
        name: BT_POWER_LED_TRIGGER,
        brightness: LED_OFF,
        registered: true,
    }
}

pub const fn bt_leds_cleanup(mut trigger: LedTrigger) -> LedTrigger {
    trigger.registered = false;
    trigger
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bluetooth_leds_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bluetooth/leds.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/bluetooth/leds.h"
        ));
        assert!(
            header.contains("void hci_leds_update_powered(struct hci_dev *hdev, bool enabled);")
        );
        assert!(source.contains("DEFINE_LED_TRIGGER(bt_power_led_trigger);"));
        assert!(source.contains("struct hci_basic_led_trigger"));
        assert!(source.contains("hci_leds_update_powered(struct hci_dev *hdev, bool enabled)"));
        assert!(source.contains("if (hdev->power_led)"));
        assert!(source.contains("enabled ? LED_FULL : LED_OFF"));
        assert!(source.contains("if (!enabled)"));
        assert!(source.contains("list_for_each_entry(d, &hci_dev_list, list)"));
        assert!(source.contains("if (test_bit(HCI_UP, &d->flags))"));
        assert!(source.contains("led_trigger_event(bt_power_led_trigger"));
        assert!(source.contains("power_activate(struct led_classdev *led_cdev)"));
        assert!(source.contains("powered = test_bit(HCI_UP, &htrig->hdev->flags);"));
        assert!(source.contains("led_set_brightness(led_cdev, powered ? LED_FULL : LED_OFF);"));
        assert!(source.contains("led_allocate_basic(struct hci_dev *hdev"));
        assert!(source.contains("devm_kasprintf(&hdev->dev, GFP_KERNEL"));
        assert!(source.contains("\"%s-%s\", hdev->name"));
        assert!(
            source
                .contains("hdev->power_led = led_allocate_basic(hdev, power_activate, \"power\");")
        );
        assert!(
            source.contains(
                "led_trigger_register_simple(\"bluetooth-power\", &bt_power_led_trigger);"
            )
        );
        assert!(source.contains("led_trigger_unregister_simple(bt_power_led_trigger);"));
    }

    #[test]
    fn hci_leds_track_per_device_and_global_power() {
        let mut bt_power = bt_leds_init();
        let mut hdev = HciDev {
            name: "hci0",
            up: true,
            power_led: None,
        };
        hci_leds_init(&mut hdev, true, true, true);
        assert_eq!(hdev.power_led.unwrap().name, "hci0-power");

        hci_leds_update_powered(&mut hdev, true, false, &mut bt_power);
        assert_eq!(hdev.power_led.unwrap().brightness, LED_FULL);
        assert_eq!(bt_power.brightness, LED_FULL);

        hci_leds_update_powered(&mut hdev, false, true, &mut bt_power);
        assert_eq!(hdev.power_led.unwrap().brightness, LED_OFF);
        assert_eq!(bt_power.brightness, LED_FULL);

        hci_leds_update_powered(&mut hdev, false, false, &mut bt_power);
        assert_eq!(bt_power.brightness, LED_OFF);
        assert!(led_allocate_basic(&hdev, false, true, true).is_none());
        let mut led = LedTrigger {
            name: "manual",
            brightness: LED_OFF,
            registered: true,
        };
        assert_eq!(power_activate(&hdev, &mut led), 0);
        assert_eq!(led.brightness, LED_FULL);
        assert!(!bt_leds_cleanup(bt_power).registered);
    }
}
