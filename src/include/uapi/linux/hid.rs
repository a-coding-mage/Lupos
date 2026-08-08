// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/hid.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016146

/*
 * Copyright (c) 1999 Andreas Gal
 * Copyright (c) 2000-2001 Vojtech Pavlik
 * Copyright (c) 2006-2007 Jiri Kosina
 */

/*
 * USB HID (Human Interface Device) interface class code
 */
pub const USB_INTERFACE_CLASS_HID: core::ffi::c_int = 3;

/*
 * USB HID interface subclass and protocol codes
 */
pub const USB_INTERFACE_SUBCLASS_BOOT: core::ffi::c_int = 1;
pub const USB_INTERFACE_PROTOCOL_KEYBOARD: core::ffi::c_int = 1;
pub const USB_INTERFACE_PROTOCOL_MOUSE: core::ffi::c_int = 2;

/*
 * HID report types --- Ouch! HID spec says 1 2 3!
 */
#[repr(C)]
pub enum hid_report_type {
    HID_INPUT_REPORT = 0,
    HID_OUTPUT_REPORT = 1,
    HID_FEATURE_REPORT = 2,
    HID_REPORT_TYPES = 3,
}

/*
 * HID class requests
 */
#[repr(C)]
pub enum hid_class_request {
    HID_REQ_GET_REPORT = 0x01,
    HID_REQ_GET_IDLE = 0x02,
    HID_REQ_GET_PROTOCOL = 0x03,
    HID_REQ_SET_REPORT = 0x09,
    HID_REQ_SET_IDLE = 0x0A,
    HID_REQ_SET_PROTOCOL = 0x0B,
}

/*
 * HID class descriptor types
 *
 * USB_TYPE_CLASS is (0x01 << 5) in include/uapi/linux/usb/ch9.h.
 */
pub const HID_DT_HID: core::ffi::c_int = ((0x01_i32) << 5) | 0x01;
pub const HID_DT_REPORT: core::ffi::c_int = ((0x01_i32) << 5) | 0x02;
pub const HID_DT_PHYSICAL: core::ffi::c_int = ((0x01_i32) << 5) | 0x03;

pub const HID_MAX_DESCRIPTOR_SIZE: core::ffi::c_int = 4096;
