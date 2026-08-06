// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/hid.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016146

// Copyright (c) 1999 Andreas Gal
// Copyright (c) 2000-2001 Vojtech Pavlik
// Copyright (c) 2006-2007 Jiri Kosina

//! USB Human Interface Device (HID) UAPI constants.

use core::ffi::c_int;

/// C `enum hid_report_type`, preserving its distinct tag and `int` ABI.
///
/// The transparent representation accepts every `c_int` bit pattern, as C
/// does for values received from an ABI boundary.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct hid_report_type(pub c_int);

/// C `enum hid_class_request`, preserving its distinct tag and `int` ABI.
///
/// The transparent representation accepts every `c_int` bit pattern, as C
/// does for values received from an ABI boundary.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct hid_class_request(pub c_int);

/// USB HID (Human Interface Device) interface class code.
pub const USB_INTERFACE_CLASS_HID: c_int = 3;

/// USB HID boot interface subclass code.
pub const USB_INTERFACE_SUBCLASS_BOOT: c_int = 1;
/// USB HID boot keyboard interface protocol code.
pub const USB_INTERFACE_PROTOCOL_KEYBOARD: c_int = 1;
/// USB HID boot mouse interface protocol code.
pub const USB_INTERFACE_PROTOCOL_MOUSE: c_int = 2;

/* HID report types.  The HID specification uses type values 1, 2, and 3;
 * Linux uses zero-based indices for its report-type array. */
pub const HID_INPUT_REPORT: hid_report_type = hid_report_type(0);
pub const HID_OUTPUT_REPORT: hid_report_type = hid_report_type(1);
pub const HID_FEATURE_REPORT: hid_report_type = hid_report_type(2);
pub const HID_REPORT_TYPES: hid_report_type = hid_report_type(3);

/// HID class requests.
pub const HID_REQ_GET_REPORT: hid_class_request = hid_class_request(0x01);
pub const HID_REQ_GET_IDLE: hid_class_request = hid_class_request(0x02);
pub const HID_REQ_GET_PROTOCOL: hid_class_request = hid_class_request(0x03);
pub const HID_REQ_SET_REPORT: hid_class_request = hid_class_request(0x09);
pub const HID_REQ_SET_IDLE: hid_class_request = hid_class_request(0x0a);
pub const HID_REQ_SET_PROTOCOL: hid_class_request = hid_class_request(0x0b);

// `USB_TYPE_CLASS` is the USB chapter-9 macro `(0x01 << 5)`.  The C HID
// header relies on its includer to provide that macro; retain its exact
// integer expression in these fully evaluated Rust constants.
pub const HID_DT_HID: c_int = (0x01 << 5) | 0x01;
pub const HID_DT_REPORT: c_int = (0x01 << 5) | 0x02;
pub const HID_DT_PHYSICAL: c_int = (0x01 << 5) | 0x03;

pub const HID_MAX_DESCRIPTOR_SIZE: c_int = 4096;
