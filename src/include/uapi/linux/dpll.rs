// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/dpll.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016105

pub const DPLL_FAMILY_NAME: &[u8; 5] = b"dpll\0";
pub const DPLL_FAMILY_VERSION: u32 = 1;

pub type dpll_mode = u32;
pub const DPLL_MODE_MANUAL: dpll_mode = 1;
pub const DPLL_MODE_AUTOMATIC: dpll_mode = 2;
pub const __DPLL_MODE_MAX: dpll_mode = 3;
pub const DPLL_MODE_MAX: dpll_mode = __DPLL_MODE_MAX - 1;

pub type dpll_lock_status = u32;
pub const DPLL_LOCK_STATUS_UNLOCKED: dpll_lock_status = 1;
pub const DPLL_LOCK_STATUS_LOCKED: dpll_lock_status = 2;
pub const DPLL_LOCK_STATUS_LOCKED_HO_ACQ: dpll_lock_status = 3;
pub const DPLL_LOCK_STATUS_HOLDOVER: dpll_lock_status = 4;
pub const __DPLL_LOCK_STATUS_MAX: dpll_lock_status = 5;
pub const DPLL_LOCK_STATUS_MAX: dpll_lock_status = __DPLL_LOCK_STATUS_MAX - 1;

pub type dpll_lock_status_error = u32;
pub const DPLL_LOCK_STATUS_ERROR_NONE: dpll_lock_status_error = 1;
pub const DPLL_LOCK_STATUS_ERROR_UNDEFINED: dpll_lock_status_error = 2;
pub const DPLL_LOCK_STATUS_ERROR_MEDIA_DOWN: dpll_lock_status_error = 3;
pub const DPLL_LOCK_STATUS_ERROR_FRACTIONAL_FREQUENCY_OFFSET_TOO_HIGH: dpll_lock_status_error = 4;
pub const __DPLL_LOCK_STATUS_ERROR_MAX: dpll_lock_status_error = 5;
pub const DPLL_LOCK_STATUS_ERROR_MAX: dpll_lock_status_error = __DPLL_LOCK_STATUS_ERROR_MAX - 1;

pub type dpll_clock_quality_level = u32;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_PRC: dpll_clock_quality_level = 1;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_SSU_A: dpll_clock_quality_level = 2;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_SSU_B: dpll_clock_quality_level = 3;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EEC1: dpll_clock_quality_level = 4;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_PRTC: dpll_clock_quality_level = 5;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EPRTC: dpll_clock_quality_level = 6;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EEEC: dpll_clock_quality_level = 7;
pub const DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EPRC: dpll_clock_quality_level = 8;
pub const __DPLL_CLOCK_QUALITY_LEVEL_MAX: dpll_clock_quality_level = 9;
pub const DPLL_CLOCK_QUALITY_LEVEL_MAX: dpll_clock_quality_level = __DPLL_CLOCK_QUALITY_LEVEL_MAX - 1;

pub const DPLL_TEMP_DIVIDER: u32 = 1000;

pub type dpll_type = u32;
pub const DPLL_TYPE_PPS: dpll_type = 1;
pub const DPLL_TYPE_EEC: dpll_type = 2;
pub const DPLL_TYPE_GENERIC: dpll_type = 3;
pub const __DPLL_TYPE_MAX: dpll_type = 4;
pub const DPLL_TYPE_MAX: dpll_type = __DPLL_TYPE_MAX - 1;

pub type dpll_pin_type = u32;
pub const DPLL_PIN_TYPE_MUX: dpll_pin_type = 1;
pub const DPLL_PIN_TYPE_EXT: dpll_pin_type = 2;
pub const DPLL_PIN_TYPE_SYNCE_ETH_PORT: dpll_pin_type = 3;
pub const DPLL_PIN_TYPE_INT_OSCILLATOR: dpll_pin_type = 4;
pub const DPLL_PIN_TYPE_GNSS: dpll_pin_type = 5;
pub const __DPLL_PIN_TYPE_MAX: dpll_pin_type = 6;
pub const DPLL_PIN_TYPE_MAX: dpll_pin_type = __DPLL_PIN_TYPE_MAX - 1;

pub type dpll_pin_direction = u32;
pub const DPLL_PIN_DIRECTION_INPUT: dpll_pin_direction = 1;
pub const DPLL_PIN_DIRECTION_OUTPUT: dpll_pin_direction = 2;
pub const __DPLL_PIN_DIRECTION_MAX: dpll_pin_direction = 3;
pub const DPLL_PIN_DIRECTION_MAX: dpll_pin_direction = __DPLL_PIN_DIRECTION_MAX - 1;

pub const DPLL_PIN_FREQUENCY_1_HZ: u32 = 1;
pub const DPLL_PIN_FREQUENCY_10_KHZ: u32 = 10_000;
pub const DPLL_PIN_FREQUENCY_77_5_KHZ: u32 = 77_500;
pub const DPLL_PIN_FREQUENCY_10_MHZ: u32 = 10_000_000;

pub type dpll_pin_state = u32;
pub const DPLL_PIN_STATE_CONNECTED: dpll_pin_state = 1;
pub const DPLL_PIN_STATE_DISCONNECTED: dpll_pin_state = 2;
pub const DPLL_PIN_STATE_SELECTABLE: dpll_pin_state = 3;
pub const __DPLL_PIN_STATE_MAX: dpll_pin_state = 4;
pub const DPLL_PIN_STATE_MAX: dpll_pin_state = __DPLL_PIN_STATE_MAX - 1;

pub type dpll_pin_operstate = u32;
pub const DPLL_PIN_OPERSTATE_ACTIVE: dpll_pin_operstate = 1;
pub const DPLL_PIN_OPERSTATE_STANDBY: dpll_pin_operstate = 2;
pub const DPLL_PIN_OPERSTATE_NO_SIGNAL: dpll_pin_operstate = 3;
pub const DPLL_PIN_OPERSTATE_QUAL_FAILED: dpll_pin_operstate = 4;
pub const __DPLL_PIN_OPERSTATE_MAX: dpll_pin_operstate = 5;
pub const DPLL_PIN_OPERSTATE_MAX: dpll_pin_operstate = __DPLL_PIN_OPERSTATE_MAX - 1;

pub type dpll_pin_capabilities = u32;
pub const DPLL_PIN_CAPABILITIES_DIRECTION_CAN_CHANGE: dpll_pin_capabilities = 1;
pub const DPLL_PIN_CAPABILITIES_PRIORITY_CAN_CHANGE: dpll_pin_capabilities = 2;
pub const DPLL_PIN_CAPABILITIES_STATE_CAN_CHANGE: dpll_pin_capabilities = 4;

pub const DPLL_PHASE_OFFSET_DIVIDER: u32 = 1000;
pub const DPLL_PIN_MEASURED_FREQUENCY_DIVIDER: u32 = 1000;

pub type dpll_feature_state = u32;
pub const DPLL_FEATURE_STATE_DISABLE: dpll_feature_state = 0;
pub const DPLL_FEATURE_STATE_ENABLE: dpll_feature_state = 1;

pub type dpll_a = u32;
pub const DPLL_A_ID: dpll_a = 1;
pub const DPLL_A_MODULE_NAME: dpll_a = 2;
pub const DPLL_A_PAD: dpll_a = 3;
pub const DPLL_A_CLOCK_ID: dpll_a = 4;
pub const DPLL_A_MODE: dpll_a = 5;
pub const DPLL_A_MODE_SUPPORTED: dpll_a = 6;
pub const DPLL_A_LOCK_STATUS: dpll_a = 7;
pub const DPLL_A_TEMP: dpll_a = 8;
pub const DPLL_A_TYPE: dpll_a = 9;
pub const DPLL_A_LOCK_STATUS_ERROR: dpll_a = 10;
pub const DPLL_A_CLOCK_QUALITY_LEVEL: dpll_a = 11;
pub const DPLL_A_PHASE_OFFSET_MONITOR: dpll_a = 12;
pub const DPLL_A_PHASE_OFFSET_AVG_FACTOR: dpll_a = 13;
pub const DPLL_A_FREQUENCY_MONITOR: dpll_a = 14;
pub const __DPLL_A_MAX: dpll_a = 15;
pub const DPLL_A_MAX: dpll_a = __DPLL_A_MAX - 1;

pub type dpll_a_pin = u32;
pub const DPLL_A_PIN_ID: dpll_a_pin = 1;
pub const DPLL_A_PIN_PARENT_ID: dpll_a_pin = 2;
pub const DPLL_A_PIN_MODULE_NAME: dpll_a_pin = 3;
pub const DPLL_A_PIN_PAD: dpll_a_pin = 4;
pub const DPLL_A_PIN_CLOCK_ID: dpll_a_pin = 5;
pub const DPLL_A_PIN_BOARD_LABEL: dpll_a_pin = 6;
pub const DPLL_A_PIN_PANEL_LABEL: dpll_a_pin = 7;
pub const DPLL_A_PIN_PACKAGE_LABEL: dpll_a_pin = 8;
pub const DPLL_A_PIN_TYPE: dpll_a_pin = 9;
pub const DPLL_A_PIN_DIRECTION: dpll_a_pin = 10;
pub const DPLL_A_PIN_FREQUENCY: dpll_a_pin = 11;
pub const DPLL_A_PIN_FREQUENCY_SUPPORTED: dpll_a_pin = 12;
pub const DPLL_A_PIN_FREQUENCY_MIN: dpll_a_pin = 13;
pub const DPLL_A_PIN_FREQUENCY_MAX: dpll_a_pin = 14;
pub const DPLL_A_PIN_PRIO: dpll_a_pin = 15;
pub const DPLL_A_PIN_STATE: dpll_a_pin = 16;
pub const DPLL_A_PIN_CAPABILITIES: dpll_a_pin = 17;
pub const DPLL_A_PIN_PARENT_DEVICE: dpll_a_pin = 18;
pub const DPLL_A_PIN_PARENT_PIN: dpll_a_pin = 19;
pub const DPLL_A_PIN_PHASE_ADJUST_MIN: dpll_a_pin = 20;
pub const DPLL_A_PIN_PHASE_ADJUST_MAX: dpll_a_pin = 21;
pub const DPLL_A_PIN_PHASE_ADJUST: dpll_a_pin = 22;
pub const DPLL_A_PIN_PHASE_OFFSET: dpll_a_pin = 23;
pub const DPLL_A_PIN_FRACTIONAL_FREQUENCY_OFFSET: dpll_a_pin = 24;
pub const DPLL_A_PIN_ESYNC_FREQUENCY: dpll_a_pin = 25;
pub const DPLL_A_PIN_ESYNC_FREQUENCY_SUPPORTED: dpll_a_pin = 26;
pub const DPLL_A_PIN_ESYNC_PULSE: dpll_a_pin = 27;
pub const DPLL_A_PIN_REFERENCE_SYNC: dpll_a_pin = 28;
pub const DPLL_A_PIN_PHASE_ADJUST_GRAN: dpll_a_pin = 29;
pub const DPLL_A_PIN_FRACTIONAL_FREQUENCY_OFFSET_PPT: dpll_a_pin = 30;
pub const DPLL_A_PIN_MEASURED_FREQUENCY: dpll_a_pin = 31;
pub const DPLL_A_PIN_OPERSTATE: dpll_a_pin = 32;
pub const __DPLL_A_PIN_MAX: dpll_a_pin = 33;
pub const DPLL_A_PIN_MAX: dpll_a_pin = __DPLL_A_PIN_MAX - 1;

pub type dpll_cmd = u32;
pub const DPLL_CMD_DEVICE_ID_GET: dpll_cmd = 1;
pub const DPLL_CMD_DEVICE_GET: dpll_cmd = 2;
pub const DPLL_CMD_DEVICE_SET: dpll_cmd = 3;
pub const DPLL_CMD_DEVICE_CREATE_NTF: dpll_cmd = 4;
pub const DPLL_CMD_DEVICE_DELETE_NTF: dpll_cmd = 5;
pub const DPLL_CMD_DEVICE_CHANGE_NTF: dpll_cmd = 6;
pub const DPLL_CMD_PIN_ID_GET: dpll_cmd = 7;
pub const DPLL_CMD_PIN_GET: dpll_cmd = 8;
pub const DPLL_CMD_PIN_SET: dpll_cmd = 9;
pub const DPLL_CMD_PIN_CREATE_NTF: dpll_cmd = 10;
pub const DPLL_CMD_PIN_DELETE_NTF: dpll_cmd = 11;
pub const DPLL_CMD_PIN_CHANGE_NTF: dpll_cmd = 12;
pub const __DPLL_CMD_MAX: dpll_cmd = 13;
pub const DPLL_CMD_MAX: dpll_cmd = __DPLL_CMD_MAX - 1;

pub const DPLL_MCGRP_MONITOR: &[u8; 8] = b"monitor\0";
