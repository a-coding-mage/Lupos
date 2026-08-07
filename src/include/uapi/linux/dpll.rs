// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/dpll.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016105

pub const DPLL_FAMILY_NAME: &str = "dpll";
pub const DPLL_FAMILY_VERSION: i32 = 1;

/// enum dpll_mode - working modes a dpll can support, differentiates if and how
/// dpll selects one of its inputs to syntonize with it.
#[repr(i32)]
pub enum dpll_mode {
    DPLL_MODE_MANUAL = 1,
    DPLL_MODE_AUTOMATIC,
    __DPLL_MODE_MAX,
    DPLL_MODE_MAX = 2,
}

/// enum dpll_lock_status - information about dpll device lock status.
#[repr(i32)]
pub enum dpll_lock_status {
    DPLL_LOCK_STATUS_UNLOCKED = 1,
    DPLL_LOCK_STATUS_LOCKED,
    DPLL_LOCK_STATUS_LOCKED_HO_ACQ,
    DPLL_LOCK_STATUS_HOLDOVER,
    __DPLL_LOCK_STATUS_MAX,
    DPLL_LOCK_STATUS_MAX = 4,
}

/// enum dpll_lock_status_error - information about a failed status change.
#[repr(i32)]
pub enum dpll_lock_status_error {
    DPLL_LOCK_STATUS_ERROR_NONE = 1,
    DPLL_LOCK_STATUS_ERROR_UNDEFINED,
    DPLL_LOCK_STATUS_ERROR_MEDIA_DOWN,
    DPLL_LOCK_STATUS_ERROR_FRACTIONAL_FREQUENCY_OFFSET_TOO_HIGH,
    __DPLL_LOCK_STATUS_ERROR_MAX,
    DPLL_LOCK_STATUS_ERROR_MAX = 4,
}

/// Level of quality of a clock device (ITU-T G.8264/Y.1364 table 11-7).
#[repr(i32)]
pub enum dpll_clock_quality_level {
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_PRC = 1,
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_SSU_A,
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_SSU_B,
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EEC1,
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_PRTC,
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EPRTC,
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EEEC,
    DPLL_CLOCK_QUALITY_LEVEL_ITU_OPT1_EPRC,
    __DPLL_CLOCK_QUALITY_LEVEL_MAX,
    DPLL_CLOCK_QUALITY_LEVEL_MAX = 8,
}

pub const DPLL_TEMP_DIVIDER: i32 = 1000;

/// enum dpll_type - type of dpll.
#[repr(i32)]
pub enum dpll_type {
    DPLL_TYPE_PPS = 1,
    DPLL_TYPE_EEC,
    DPLL_TYPE_GENERIC,
    __DPLL_TYPE_MAX,
    DPLL_TYPE_MAX = 3,
}

/// enum dpll_pin_type - possible types of a pin.
#[repr(i32)]
pub enum dpll_pin_type {
    DPLL_PIN_TYPE_MUX = 1,
    DPLL_PIN_TYPE_EXT,
    DPLL_PIN_TYPE_SYNCE_ETH_PORT,
    DPLL_PIN_TYPE_INT_OSCILLATOR,
    DPLL_PIN_TYPE_GNSS,
    __DPLL_PIN_TYPE_MAX,
    DPLL_PIN_TYPE_MAX = 5,
}

/// enum dpll_pin_direction - possible direction of a pin.
#[repr(i32)]
pub enum dpll_pin_direction {
    DPLL_PIN_DIRECTION_INPUT = 1,
    DPLL_PIN_DIRECTION_OUTPUT,
    __DPLL_PIN_DIRECTION_MAX,
    DPLL_PIN_DIRECTION_MAX = 2,
}

pub const DPLL_PIN_FREQUENCY_1_HZ: i32 = 1;
pub const DPLL_PIN_FREQUENCY_10_KHZ: i32 = 10000;
pub const DPLL_PIN_FREQUENCY_77_5_KHZ: i32 = 77500;
pub const DPLL_PIN_FREQUENCY_10_MHZ: i32 = 10000000;

/// enum dpll_pin_state - possible states of a pin.
#[repr(i32)]
pub enum dpll_pin_state {
    DPLL_PIN_STATE_CONNECTED = 1,
    DPLL_PIN_STATE_DISCONNECTED,
    DPLL_PIN_STATE_SELECTABLE,
    __DPLL_PIN_STATE_MAX,
    DPLL_PIN_STATE_MAX = 3,
}

/// enum dpll_pin_operstate - operational state of a pin with respect to its
/// parent DPLL device.
#[repr(i32)]
pub enum dpll_pin_operstate {
    DPLL_PIN_OPERSTATE_ACTIVE = 1,
    DPLL_PIN_OPERSTATE_STANDBY,
    DPLL_PIN_OPERSTATE_NO_SIGNAL,
    DPLL_PIN_OPERSTATE_QUAL_FAILED,
    __DPLL_PIN_OPERSTATE_MAX,
    DPLL_PIN_OPERSTATE_MAX = 4,
}

/// enum dpll_pin_capabilities - capabilities represented as flags.
#[repr(i32)]
pub enum dpll_pin_capabilities {
    DPLL_PIN_CAPABILITIES_DIRECTION_CAN_CHANGE = 1,
    DPLL_PIN_CAPABILITIES_PRIORITY_CAN_CHANGE = 2,
    DPLL_PIN_CAPABILITIES_STATE_CAN_CHANGE = 4,
}

pub const DPLL_PHASE_OFFSET_DIVIDER: i32 = 1000;
pub const DPLL_PIN_MEASURED_FREQUENCY_DIVIDER: i32 = 1000;

/// enum dpll_feature_state - control and status state of a feature.
#[repr(i32)]
pub enum dpll_feature_state {
    DPLL_FEATURE_STATE_DISABLE,
    DPLL_FEATURE_STATE_ENABLE,
}

#[repr(i32)]
pub enum dpll_a {
    DPLL_A_ID = 1,
    DPLL_A_MODULE_NAME,
    DPLL_A_PAD,
    DPLL_A_CLOCK_ID,
    DPLL_A_MODE,
    DPLL_A_MODE_SUPPORTED,
    DPLL_A_LOCK_STATUS,
    DPLL_A_TEMP,
    DPLL_A_TYPE,
    DPLL_A_LOCK_STATUS_ERROR,
    DPLL_A_CLOCK_QUALITY_LEVEL,
    DPLL_A_PHASE_OFFSET_MONITOR,
    DPLL_A_PHASE_OFFSET_AVG_FACTOR,
    DPLL_A_FREQUENCY_MONITOR,
    __DPLL_A_MAX,
    DPLL_A_MAX = 14,
}

#[repr(i32)]
pub enum dpll_a_pin {
    DPLL_A_PIN_ID = 1,
    DPLL_A_PIN_PARENT_ID,
    DPLL_A_PIN_MODULE_NAME,
    DPLL_A_PIN_PAD,
    DPLL_A_PIN_CLOCK_ID,
    DPLL_A_PIN_BOARD_LABEL,
    DPLL_A_PIN_PANEL_LABEL,
    DPLL_A_PIN_PACKAGE_LABEL,
    DPLL_A_PIN_TYPE,
    DPLL_A_PIN_DIRECTION,
    DPLL_A_PIN_FREQUENCY,
    DPLL_A_PIN_FREQUENCY_SUPPORTED,
    DPLL_A_PIN_FREQUENCY_MIN,
    DPLL_A_PIN_FREQUENCY_MAX,
    DPLL_A_PIN_PRIO,
    DPLL_A_PIN_STATE,
    DPLL_A_PIN_CAPABILITIES,
    DPLL_A_PIN_PARENT_DEVICE,
    DPLL_A_PIN_PARENT_PIN,
    DPLL_A_PIN_PHASE_ADJUST_MIN,
    DPLL_A_PIN_PHASE_ADJUST_MAX,
    DPLL_A_PIN_PHASE_ADJUST,
    DPLL_A_PIN_PHASE_OFFSET,
    DPLL_A_PIN_FRACTIONAL_FREQUENCY_OFFSET,
    DPLL_A_PIN_ESYNC_FREQUENCY,
    DPLL_A_PIN_ESYNC_FREQUENCY_SUPPORTED,
    DPLL_A_PIN_ESYNC_PULSE,
    DPLL_A_PIN_REFERENCE_SYNC,
    DPLL_A_PIN_PHASE_ADJUST_GRAN,
    DPLL_A_PIN_FRACTIONAL_FREQUENCY_OFFSET_PPT,
    DPLL_A_PIN_MEASURED_FREQUENCY,
    DPLL_A_PIN_OPERSTATE,
    __DPLL_A_PIN_MAX,
    DPLL_A_PIN_MAX = 32,
}

#[repr(i32)]
pub enum dpll_cmd {
    DPLL_CMD_DEVICE_ID_GET = 1,
    DPLL_CMD_DEVICE_GET,
    DPLL_CMD_DEVICE_SET,
    DPLL_CMD_DEVICE_CREATE_NTF,
    DPLL_CMD_DEVICE_DELETE_NTF,
    DPLL_CMD_DEVICE_CHANGE_NTF,
    DPLL_CMD_PIN_ID_GET,
    DPLL_CMD_PIN_GET,
    DPLL_CMD_PIN_SET,
    DPLL_CMD_PIN_CREATE_NTF,
    DPLL_CMD_PIN_DELETE_NTF,
    DPLL_CMD_PIN_CHANGE_NTF,
    __DPLL_CMD_MAX,
    DPLL_CMD_MAX = 12,
}

pub const DPLL_MCGRP_MONITOR: &str = "monitor";
