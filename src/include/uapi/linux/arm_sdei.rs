/* SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note */
/* Copyright (C) 2017 Arm Ltd. */
//! linux-source: include/uapi/linux/arm_sdei.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016053

pub const SDEI_1_0_FN_BASE: u32 = 0xC4000020;
pub const SDEI_1_0_MASK: u32 = 0xFFFFFFE0;

#[macro_export]
macro_rules! SDEI_1_0_FN {
    ($n:expr) => {
        0xC4000020 + ($n)
    };
}

pub const SDEI_1_0_FN_SDEI_VERSION: u32 = SDEI_1_0_FN!(0x00u32);
pub const SDEI_1_0_FN_SDEI_EVENT_REGISTER: u32 = SDEI_1_0_FN!(0x01u32);
pub const SDEI_1_0_FN_SDEI_EVENT_ENABLE: u32 = SDEI_1_0_FN!(0x02u32);
pub const SDEI_1_0_FN_SDEI_EVENT_DISABLE: u32 = SDEI_1_0_FN!(0x03u32);
pub const SDEI_1_0_FN_SDEI_EVENT_CONTEXT: u32 = SDEI_1_0_FN!(0x04u32);
pub const SDEI_1_0_FN_SDEI_EVENT_COMPLETE: u32 = SDEI_1_0_FN!(0x05u32);
pub const SDEI_1_0_FN_SDEI_EVENT_COMPLETE_AND_RESUME: u32 = SDEI_1_0_FN!(0x06u32);
pub const SDEI_1_0_FN_SDEI_EVENT_UNREGISTER: u32 = SDEI_1_0_FN!(0x07u32);
pub const SDEI_1_0_FN_SDEI_EVENT_STATUS: u32 = SDEI_1_0_FN!(0x08u32);
pub const SDEI_1_0_FN_SDEI_EVENT_GET_INFO: u32 = SDEI_1_0_FN!(0x09u32);
pub const SDEI_1_0_FN_SDEI_EVENT_ROUTING_SET: u32 = SDEI_1_0_FN!(0x0Au32);
pub const SDEI_1_0_FN_SDEI_PE_MASK: u32 = SDEI_1_0_FN!(0x0Bu32);
pub const SDEI_1_0_FN_SDEI_PE_UNMASK: u32 = SDEI_1_0_FN!(0x0Cu32);
pub const SDEI_1_0_FN_SDEI_INTERRUPT_BIND: u32 = SDEI_1_0_FN!(0x0Du32);
pub const SDEI_1_0_FN_SDEI_INTERRUPT_RELEASE: u32 = SDEI_1_0_FN!(0x0Eu32);
pub const SDEI_1_0_FN_SDEI_PRIVATE_RESET: u32 = SDEI_1_0_FN!(0x11u32);
pub const SDEI_1_0_FN_SDEI_SHARED_RESET: u32 = SDEI_1_0_FN!(0x12u32);

pub const SDEI_VERSION_MAJOR_SHIFT: u32 = 48;
pub const SDEI_VERSION_MAJOR_MASK: u64 = 0x7fff;
pub const SDEI_VERSION_MINOR_SHIFT: u32 = 32;
pub const SDEI_VERSION_MINOR_MASK: u64 = 0xffff;
pub const SDEI_VERSION_VENDOR_SHIFT: u32 = 0;
pub const SDEI_VERSION_VENDOR_MASK: u64 = 0xffffffff;

#[macro_export]
macro_rules! SDEI_VERSION_MAJOR {
    ($x:expr) => {
        (($x) >> 48 & 0x7fff)
    };
}

#[macro_export]
macro_rules! SDEI_VERSION_MINOR {
    ($x:expr) => {
        (($x) >> 32 & 0xffff)
    };
}

#[macro_export]
macro_rules! SDEI_VERSION_VENDOR {
    ($x:expr) => {
        (($x) >> 0 & 0xffffffff)
    };
}

pub const SDEI_SUCCESS: i32 = 0;
pub const SDEI_NOT_SUPPORTED: i32 = -1;
pub const SDEI_INVALID_PARAMETERS: i32 = -2;
pub const SDEI_DENIED: i32 = -3;
pub const SDEI_PENDING: i32 = -5;
pub const SDEI_OUT_OF_RESOURCE: i32 = -10;

pub const SDEI_EVENT_REGISTER_RM_ANY: u32 = 0;
pub const SDEI_EVENT_REGISTER_RM_PE: u32 = 1;

pub const SDEI_EVENT_STATUS_RUNNING: u32 = 2;
pub const SDEI_EVENT_STATUS_ENABLED: u32 = 1;
pub const SDEI_EVENT_STATUS_REGISTERED: u32 = 0;

pub const SDEI_EV_HANDLED: u32 = 0;
pub const SDEI_EV_FAILED: u32 = 1;

pub const SDEI_EVENT_INFO_EV_TYPE: u32 = 0;
pub const SDEI_EVENT_INFO_EV_SIGNALED: u32 = 1;
pub const SDEI_EVENT_INFO_EV_PRIORITY: u32 = 2;
pub const SDEI_EVENT_INFO_EV_ROUTING_MODE: u32 = 3;
pub const SDEI_EVENT_INFO_EV_ROUTING_AFF: u32 = 4;

pub const SDEI_EVENT_TYPE_PRIVATE: u32 = 0;
pub const SDEI_EVENT_TYPE_SHARED: u32 = 1;
pub const SDEI_EVENT_PRIORITY_NORMAL: u32 = 0;
pub const SDEI_EVENT_PRIORITY_CRITICAL: u32 = 1;
