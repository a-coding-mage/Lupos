// SPDX-License-Identifier: (GPL-2.0 OR BSD-2-Clause)
//! linux-source: include/dt-bindings/leds/common.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013171

// Copyright (C) 2015, Samsung Electronics Co., Ltd.
// Author: Jacek Anaszewski <j.anaszewski@samsung.com>
// Copyright (C) 2019 Jacek Anaszewski <jacek.anaszewski@gmail.com>
// Copyright (C) 2020 Pavel Machek <pavel@ucw.cz>

use core::ffi::c_int;

// External trigger type.
pub const LEDS_TRIG_TYPE_EDGE: c_int = 0;
pub const LEDS_TRIG_TYPE_LEVEL: c_int = 1;

// Boost modes.
pub const LEDS_BOOST_OFF: c_int = 0;
pub const LEDS_BOOST_ADAPTIVE: c_int = 1;
pub const LEDS_BOOST_FIXED: c_int = 2;

// Standard LED colors.
pub const LED_COLOR_ID_WHITE: c_int = 0;
pub const LED_COLOR_ID_RED: c_int = 1;
pub const LED_COLOR_ID_GREEN: c_int = 2;
pub const LED_COLOR_ID_BLUE: c_int = 3;
pub const LED_COLOR_ID_AMBER: c_int = 4;
pub const LED_COLOR_ID_VIOLET: c_int = 5;
pub const LED_COLOR_ID_YELLOW: c_int = 6;
pub const LED_COLOR_ID_IR: c_int = 7;
pub const LED_COLOR_ID_MULTI: c_int = 8;
pub const LED_COLOR_ID_RGB: c_int = 9;
pub const LED_COLOR_ID_PURPLE: c_int = 10;
pub const LED_COLOR_ID_ORANGE: c_int = 11;
pub const LED_COLOR_ID_PINK: c_int = 12;
pub const LED_COLOR_ID_CYAN: c_int = 13;
pub const LED_COLOR_ID_LIME: c_int = 14;
pub const LED_COLOR_ID_MAX: c_int = 15;

// Standard LED functions.
pub const LED_FUNCTION_CAPSLOCK: &[u8; 9] = b"capslock\0";
pub const LED_FUNCTION_SCROLLLOCK: &[u8; 11] = b"scrolllock\0";
pub const LED_FUNCTION_NUMLOCK: &[u8; 8] = b"numlock\0";
pub const LED_FUNCTION_FNLOCK: &[u8; 7] = b"fnlock\0";
pub const LED_FUNCTION_KBD_BACKLIGHT: &[u8; 14] = b"kbd_backlight\0";

pub const LED_FUNCTION_POWER: &[u8; 6] = b"power\0";
pub const LED_FUNCTION_DISK: &[u8; 5] = b"disk\0";
pub const LED_FUNCTION_CHARGING: &[u8; 9] = b"charging\0";
pub const LED_FUNCTION_STATUS: &[u8; 7] = b"status\0";
pub const LED_FUNCTION_MICMUTE: &[u8; 8] = b"micmute\0";
pub const LED_FUNCTION_MUTE: &[u8; 5] = b"mute\0";

pub const LED_FUNCTION_PLAYER1: &[u8; 9] = b"player-1\0";
pub const LED_FUNCTION_PLAYER2: &[u8; 9] = b"player-2\0";
pub const LED_FUNCTION_PLAYER3: &[u8; 9] = b"player-3\0";
pub const LED_FUNCTION_PLAYER4: &[u8; 9] = b"player-4\0";
pub const LED_FUNCTION_PLAYER5: &[u8; 9] = b"player-5\0";

pub const LED_FUNCTION_ACTIVITY: &[u8; 9] = b"activity\0";
pub const LED_FUNCTION_ALARM: &[u8; 6] = b"alarm\0";
pub const LED_FUNCTION_BACKLIGHT: &[u8; 10] = b"backlight\0";
pub const LED_FUNCTION_BLUETOOTH: &[u8; 10] = b"bluetooth\0";
pub const LED_FUNCTION_BOOT: &[u8; 5] = b"boot\0";
pub const LED_FUNCTION_CPU: &[u8; 4] = b"cpu\0";
pub const LED_FUNCTION_DEBUG: &[u8; 6] = b"debug\0";
pub const LED_FUNCTION_DISK_ACTIVITY: &[u8; 14] = b"disk-activity\0";
pub const LED_FUNCTION_DISK_ERR: &[u8; 9] = b"disk-err\0";
pub const LED_FUNCTION_DISK_READ: &[u8; 10] = b"disk-read\0";
pub const LED_FUNCTION_DISK_WRITE: &[u8; 11] = b"disk-write\0";
pub const LED_FUNCTION_FAULT: &[u8; 6] = b"fault\0";
pub const LED_FUNCTION_FLASH: &[u8; 6] = b"flash\0";
pub const LED_FUNCTION_HEARTBEAT: &[u8; 10] = b"heartbeat\0";
pub const LED_FUNCTION_INDICATOR: &[u8; 10] = b"indicator\0";
pub const LED_FUNCTION_LAN: &[u8; 4] = b"lan\0";
pub const LED_FUNCTION_MAIL: &[u8; 5] = b"mail\0";
pub const LED_FUNCTION_MOBILE: &[u8; 7] = b"mobile\0";
pub const LED_FUNCTION_MTD: &[u8; 4] = b"mtd\0";
pub const LED_FUNCTION_PANIC: &[u8; 6] = b"panic\0";
pub const LED_FUNCTION_PROGRAMMING: &[u8; 12] = b"programming\0";
pub const LED_FUNCTION_RX: &[u8; 3] = b"rx\0";
pub const LED_FUNCTION_SD: &[u8; 3] = b"sd\0";
pub const LED_FUNCTION_SPEED_LAN: &[u8; 10] = b"speed-lan\0";
pub const LED_FUNCTION_SPEED_WAN: &[u8; 10] = b"speed-wan\0";
pub const LED_FUNCTION_STANDBY: &[u8; 8] = b"standby\0";
pub const LED_FUNCTION_TORCH: &[u8; 6] = b"torch\0";
pub const LED_FUNCTION_TX: &[u8; 3] = b"tx\0";
pub const LED_FUNCTION_USB: &[u8; 4] = b"usb\0";
pub const LED_FUNCTION_WAN: &[u8; 4] = b"wan\0";
pub const LED_FUNCTION_WAN_ONLINE: &[u8; 11] = b"wan-online\0";
pub const LED_FUNCTION_WLAN: &[u8; 5] = b"wlan\0";
pub const LED_FUNCTION_WLAN_2GHZ: &[u8; 10] = b"wlan-2ghz\0";
pub const LED_FUNCTION_WLAN_5GHZ: &[u8; 10] = b"wlan-5ghz\0";
pub const LED_FUNCTION_WLAN_6GHZ: &[u8; 10] = b"wlan-6ghz\0";
pub const LED_FUNCTION_WPS: &[u8; 4] = b"wps\0";
