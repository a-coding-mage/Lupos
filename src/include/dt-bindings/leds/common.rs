// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/dt-bindings/leds/common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013171

// Copyright (C) 2015 Samsung Electronics Co., Ltd.
// Copyright (C) 2019 Jacek Anaszewski <jacek.anaszewski@gmail.com>
// Copyright (C) 2020 Pavel Machek <pavel@ucw.cz>

/// External trigger type.
pub const LEDS_TRIG_TYPE_EDGE: u32 = 0;
pub const LEDS_TRIG_TYPE_LEVEL: u32 = 1;

/// Boost modes.
pub const LEDS_BOOST_OFF: u32 = 0;
pub const LEDS_BOOST_ADAPTIVE: u32 = 1;
pub const LEDS_BOOST_FIXED: u32 = 2;

/// Standard LED colors.
pub const LED_COLOR_ID_WHITE: u32 = 0;
pub const LED_COLOR_ID_RED: u32 = 1;
pub const LED_COLOR_ID_GREEN: u32 = 2;
pub const LED_COLOR_ID_BLUE: u32 = 3;
pub const LED_COLOR_ID_AMBER: u32 = 4;
pub const LED_COLOR_ID_VIOLET: u32 = 5;
pub const LED_COLOR_ID_YELLOW: u32 = 6;
pub const LED_COLOR_ID_IR: u32 = 7;
pub const LED_COLOR_ID_MULTI: u32 = 8;
pub const LED_COLOR_ID_RGB: u32 = 9;
pub const LED_COLOR_ID_PURPLE: u32 = 10;
pub const LED_COLOR_ID_ORANGE: u32 = 11;
pub const LED_COLOR_ID_PINK: u32 = 12;
pub const LED_COLOR_ID_CYAN: u32 = 13;
pub const LED_COLOR_ID_LIME: u32 = 14;
pub const LED_COLOR_ID_MAX: u32 = 15;

/// Standard LED functions.
pub const LED_FUNCTION_CAPSLOCK: &str = "capslock";
pub const LED_FUNCTION_SCROLLLOCK: &str = "scrolllock";
pub const LED_FUNCTION_NUMLOCK: &str = "numlock";
pub const LED_FUNCTION_FNLOCK: &str = "fnlock";
pub const LED_FUNCTION_KBD_BACKLIGHT: &str = "kbd_backlight";
pub const LED_FUNCTION_POWER: &str = "power";
pub const LED_FUNCTION_DISK: &str = "disk";
pub const LED_FUNCTION_CHARGING: &str = "charging";
pub const LED_FUNCTION_STATUS: &str = "status";
pub const LED_FUNCTION_MICMUTE: &str = "micmute";
pub const LED_FUNCTION_MUTE: &str = "mute";
pub const LED_FUNCTION_PLAYER1: &str = "player-1";
pub const LED_FUNCTION_PLAYER2: &str = "player-2";
pub const LED_FUNCTION_PLAYER3: &str = "player-3";
pub const LED_FUNCTION_PLAYER4: &str = "player-4";
pub const LED_FUNCTION_PLAYER5: &str = "player-5";
pub const LED_FUNCTION_ACTIVITY: &str = "activity";
pub const LED_FUNCTION_ALARM: &str = "alarm";
pub const LED_FUNCTION_BACKLIGHT: &str = "backlight";
pub const LED_FUNCTION_BLUETOOTH: &str = "bluetooth";
pub const LED_FUNCTION_BOOT: &str = "boot";
pub const LED_FUNCTION_CPU: &str = "cpu";
pub const LED_FUNCTION_DEBUG: &str = "debug";
pub const LED_FUNCTION_DISK_ACTIVITY: &str = "disk-activity";
pub const LED_FUNCTION_DISK_ERR: &str = "disk-err";
pub const LED_FUNCTION_DISK_READ: &str = "disk-read";
pub const LED_FUNCTION_DISK_WRITE: &str = "disk-write";
pub const LED_FUNCTION_FAULT: &str = "fault";
pub const LED_FUNCTION_FLASH: &str = "flash";
pub const LED_FUNCTION_HEARTBEAT: &str = "heartbeat";
pub const LED_FUNCTION_INDICATOR: &str = "indicator";
pub const LED_FUNCTION_LAN: &str = "lan";
pub const LED_FUNCTION_MAIL: &str = "mail";
pub const LED_FUNCTION_MOBILE: &str = "mobile";
pub const LED_FUNCTION_MTD: &str = "mtd";
pub const LED_FUNCTION_PANIC: &str = "panic";
pub const LED_FUNCTION_PROGRAMMING: &str = "programming";
pub const LED_FUNCTION_RX: &str = "rx";
pub const LED_FUNCTION_SD: &str = "sd";
pub const LED_FUNCTION_SPEED_LAN: &str = "speed-lan";
pub const LED_FUNCTION_SPEED_WAN: &str = "speed-wan";
pub const LED_FUNCTION_STANDBY: &str = "standby";
pub const LED_FUNCTION_TORCH: &str = "torch";
pub const LED_FUNCTION_TX: &str = "tx";
pub const LED_FUNCTION_USB: &str = "usb";
pub const LED_FUNCTION_WAN: &str = "wan";
pub const LED_FUNCTION_WAN_ONLINE: &str = "wan-online";
pub const LED_FUNCTION_WLAN: &str = "wlan";
pub const LED_FUNCTION_WLAN_2GHZ: &str = "wlan-2ghz";
pub const LED_FUNCTION_WLAN_5GHZ: &str = "wlan-5ghz";
pub const LED_FUNCTION_WLAN_6GHZ: &str = "wlan-6ghz";
pub const LED_FUNCTION_WPS: &str = "wps";
