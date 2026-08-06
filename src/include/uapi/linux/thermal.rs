// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/thermal.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016417

//! Thermal generic-netlink UAPI definitions.

use core::ffi::{c_char, c_int};

macro_rules! thermal_uapi_enum {
    ($name:ident) => {
        /// C `enum` tag represented with the frozen C `int` ABI.
        ///
        /// The transparent representation accepts every `c_int` bit pattern,
        /// including values received from the UAPI that are not named here.
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct $name(pub c_int);
    };
}

thermal_uapi_enum!(thermal_device_mode);
thermal_uapi_enum!(thermal_trip_type);
thermal_uapi_enum!(thermal_genl_attr);
thermal_uapi_enum!(thermal_genl_sampling);
thermal_uapi_enum!(thermal_genl_event);
thermal_uapi_enum!(thermal_genl_cmd);

pub const THERMAL_NAME_LENGTH: c_int = 20;
pub const THERMAL_THRESHOLD_WAY_UP: c_int = 0x1;
pub const THERMAL_THRESHOLD_WAY_DOWN: c_int = 0x2;

pub const THERMAL_DEVICE_DISABLED: thermal_device_mode = thermal_device_mode(0);
pub const THERMAL_DEVICE_ENABLED: thermal_device_mode = thermal_device_mode(1);

pub const THERMAL_TRIP_ACTIVE: thermal_trip_type = thermal_trip_type(0);
pub const THERMAL_TRIP_PASSIVE: thermal_trip_type = thermal_trip_type(1);
pub const THERMAL_TRIP_HOT: thermal_trip_type = thermal_trip_type(2);
pub const THERMAL_TRIP_CRITICAL: thermal_trip_type = thermal_trip_type(3);

// C string-literal macros have static storage and decay to `const char *` in
// ordinary expression context.  Keep the backing NUL-terminated arrays so
// callers can retain that same decay with `.as_ptr()`.
pub static THERMAL_GENL_FAMILY_NAME: [c_char; 8] = [
    b't' as c_char, b'h' as c_char, b'e' as c_char, b'r' as c_char,
    b'm' as c_char, b'a' as c_char, b'l' as c_char, 0,
];
pub const THERMAL_GENL_VERSION: c_int = 0x02;
pub static THERMAL_GENL_SAMPLING_GROUP_NAME: [c_char; 9] = [
    b's' as c_char, b'a' as c_char, b'm' as c_char, b'p' as c_char,
    b'l' as c_char, b'i' as c_char, b'n' as c_char, b'g' as c_char, 0,
];
pub static THERMAL_GENL_EVENT_GROUP_NAME: [c_char; 6] = [
    b'e' as c_char, b'v' as c_char, b'e' as c_char, b'n' as c_char,
    b't' as c_char, 0,
];

pub const THERMAL_GENL_ATTR_UNSPEC: thermal_genl_attr = thermal_genl_attr(0);
pub const THERMAL_GENL_ATTR_TZ: thermal_genl_attr = thermal_genl_attr(1);
pub const THERMAL_GENL_ATTR_TZ_ID: thermal_genl_attr = thermal_genl_attr(2);
pub const THERMAL_GENL_ATTR_TZ_TEMP: thermal_genl_attr = thermal_genl_attr(3);
pub const THERMAL_GENL_ATTR_TZ_TRIP: thermal_genl_attr = thermal_genl_attr(4);
pub const THERMAL_GENL_ATTR_TZ_TRIP_ID: thermal_genl_attr = thermal_genl_attr(5);
pub const THERMAL_GENL_ATTR_TZ_TRIP_TYPE: thermal_genl_attr = thermal_genl_attr(6);
pub const THERMAL_GENL_ATTR_TZ_TRIP_TEMP: thermal_genl_attr = thermal_genl_attr(7);
pub const THERMAL_GENL_ATTR_TZ_TRIP_HYST: thermal_genl_attr = thermal_genl_attr(8);
pub const THERMAL_GENL_ATTR_TZ_MODE: thermal_genl_attr = thermal_genl_attr(9);
pub const THERMAL_GENL_ATTR_TZ_NAME: thermal_genl_attr = thermal_genl_attr(10);
pub const THERMAL_GENL_ATTR_TZ_CDEV_WEIGHT: thermal_genl_attr = thermal_genl_attr(11);
pub const THERMAL_GENL_ATTR_TZ_GOV: thermal_genl_attr = thermal_genl_attr(12);
pub const THERMAL_GENL_ATTR_TZ_GOV_NAME: thermal_genl_attr = thermal_genl_attr(13);
pub const THERMAL_GENL_ATTR_CDEV: thermal_genl_attr = thermal_genl_attr(14);
pub const THERMAL_GENL_ATTR_CDEV_ID: thermal_genl_attr = thermal_genl_attr(15);
pub const THERMAL_GENL_ATTR_CDEV_CUR_STATE: thermal_genl_attr = thermal_genl_attr(16);
pub const THERMAL_GENL_ATTR_CDEV_MAX_STATE: thermal_genl_attr = thermal_genl_attr(17);
pub const THERMAL_GENL_ATTR_CDEV_NAME: thermal_genl_attr = thermal_genl_attr(18);
pub const THERMAL_GENL_ATTR_GOV_NAME: thermal_genl_attr = thermal_genl_attr(19);
pub const THERMAL_GENL_ATTR_CPU_CAPABILITY: thermal_genl_attr = thermal_genl_attr(20);
pub const THERMAL_GENL_ATTR_CPU_CAPABILITY_ID: thermal_genl_attr = thermal_genl_attr(21);
pub const THERMAL_GENL_ATTR_CPU_CAPABILITY_PERFORMANCE: thermal_genl_attr = thermal_genl_attr(22);
pub const THERMAL_GENL_ATTR_CPU_CAPABILITY_EFFICIENCY: thermal_genl_attr = thermal_genl_attr(23);
pub const THERMAL_GENL_ATTR_THRESHOLD: thermal_genl_attr = thermal_genl_attr(24);
pub const THERMAL_GENL_ATTR_THRESHOLD_TEMP: thermal_genl_attr = thermal_genl_attr(25);
pub const THERMAL_GENL_ATTR_THRESHOLD_DIRECTION: thermal_genl_attr = thermal_genl_attr(26);
pub const THERMAL_GENL_ATTR_TZ_PREV_TEMP: thermal_genl_attr = thermal_genl_attr(27);
pub const __THERMAL_GENL_ATTR_MAX: thermal_genl_attr = thermal_genl_attr(28);
pub const THERMAL_GENL_ATTR_MAX: thermal_genl_attr =
    thermal_genl_attr(__THERMAL_GENL_ATTR_MAX.0 - 1);

pub const THERMAL_GENL_SAMPLING_TEMP: thermal_genl_sampling = thermal_genl_sampling(0);
pub const __THERMAL_GENL_SAMPLING_MAX: thermal_genl_sampling = thermal_genl_sampling(1);
pub const THERMAL_GENL_SAMPLING_MAX: thermal_genl_sampling =
    thermal_genl_sampling(__THERMAL_GENL_SAMPLING_MAX.0 - 1);

pub const THERMAL_GENL_EVENT_UNSPEC: thermal_genl_event = thermal_genl_event(0);
pub const THERMAL_GENL_EVENT_TZ_CREATE: thermal_genl_event = thermal_genl_event(1);
pub const THERMAL_GENL_EVENT_TZ_DELETE: thermal_genl_event = thermal_genl_event(2);
pub const THERMAL_GENL_EVENT_TZ_DISABLE: thermal_genl_event = thermal_genl_event(3);
pub const THERMAL_GENL_EVENT_TZ_ENABLE: thermal_genl_event = thermal_genl_event(4);
pub const THERMAL_GENL_EVENT_TZ_TRIP_UP: thermal_genl_event = thermal_genl_event(5);
pub const THERMAL_GENL_EVENT_TZ_TRIP_DOWN: thermal_genl_event = thermal_genl_event(6);
pub const THERMAL_GENL_EVENT_TZ_TRIP_CHANGE: thermal_genl_event = thermal_genl_event(7);
pub const THERMAL_GENL_EVENT_TZ_TRIP_ADD: thermal_genl_event = thermal_genl_event(8);
pub const THERMAL_GENL_EVENT_TZ_TRIP_DELETE: thermal_genl_event = thermal_genl_event(9);
pub const THERMAL_GENL_EVENT_CDEV_ADD: thermal_genl_event = thermal_genl_event(10);
pub const THERMAL_GENL_EVENT_CDEV_DELETE: thermal_genl_event = thermal_genl_event(11);
pub const THERMAL_GENL_EVENT_CDEV_STATE_UPDATE: thermal_genl_event = thermal_genl_event(12);
pub const THERMAL_GENL_EVENT_TZ_GOV_CHANGE: thermal_genl_event = thermal_genl_event(13);
pub const THERMAL_GENL_EVENT_CPU_CAPABILITY_CHANGE: thermal_genl_event = thermal_genl_event(14);
pub const THERMAL_GENL_EVENT_THRESHOLD_ADD: thermal_genl_event = thermal_genl_event(15);
pub const THERMAL_GENL_EVENT_THRESHOLD_DELETE: thermal_genl_event = thermal_genl_event(16);
pub const THERMAL_GENL_EVENT_THRESHOLD_FLUSH: thermal_genl_event = thermal_genl_event(17);
pub const THERMAL_GENL_EVENT_THRESHOLD_UP: thermal_genl_event = thermal_genl_event(18);
pub const THERMAL_GENL_EVENT_THRESHOLD_DOWN: thermal_genl_event = thermal_genl_event(19);
pub const __THERMAL_GENL_EVENT_MAX: thermal_genl_event = thermal_genl_event(20);
pub const THERMAL_GENL_EVENT_MAX: thermal_genl_event =
    thermal_genl_event(__THERMAL_GENL_EVENT_MAX.0 - 1);

pub const THERMAL_GENL_CMD_UNSPEC: thermal_genl_cmd = thermal_genl_cmd(0);
pub const THERMAL_GENL_CMD_TZ_GET_ID: thermal_genl_cmd = thermal_genl_cmd(1);
pub const THERMAL_GENL_CMD_TZ_GET_TRIP: thermal_genl_cmd = thermal_genl_cmd(2);
pub const THERMAL_GENL_CMD_TZ_GET_TEMP: thermal_genl_cmd = thermal_genl_cmd(3);
pub const THERMAL_GENL_CMD_TZ_GET_GOV: thermal_genl_cmd = thermal_genl_cmd(4);
pub const THERMAL_GENL_CMD_TZ_GET_MODE: thermal_genl_cmd = thermal_genl_cmd(5);
pub const THERMAL_GENL_CMD_CDEV_GET: thermal_genl_cmd = thermal_genl_cmd(6);
pub const THERMAL_GENL_CMD_THRESHOLD_GET: thermal_genl_cmd = thermal_genl_cmd(7);
pub const THERMAL_GENL_CMD_THRESHOLD_ADD: thermal_genl_cmd = thermal_genl_cmd(8);
pub const THERMAL_GENL_CMD_THRESHOLD_DELETE: thermal_genl_cmd = thermal_genl_cmd(9);
pub const THERMAL_GENL_CMD_THRESHOLD_FLUSH: thermal_genl_cmd = thermal_genl_cmd(10);
pub const __THERMAL_GENL_CMD_MAX: thermal_genl_cmd = thermal_genl_cmd(11);
pub const THERMAL_GENL_CMD_MAX: thermal_genl_cmd =
    thermal_genl_cmd(__THERMAL_GENL_CMD_MAX.0 - 1);
