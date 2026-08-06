// SPDX-License-Identifier: MIT
//! linux-source: include/xen/interface/io/xenbus.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016582

// Copyright (C) 2005 XenSource Ltd.

/// The state of either Xenbus endpoint during bus initialization.
///
/// This is represented as a transparent C `int` newtype rather than a Rust
/// enum so values outside the named protocol states retain the representation
/// and value domain that C permits for an `enum xenbus_state` object.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct xenbus_state(pub i32);

// C places enumerators in the including translation unit's ordinary
// identifier namespace, independently of the `enum xenbus_state` tag.
#[allow(non_upper_case_globals)]
pub const XenbusStateUnknown: xenbus_state = xenbus_state(0);
#[allow(non_upper_case_globals)]
pub const XenbusStateInitialising: xenbus_state = xenbus_state(1);
#[allow(non_upper_case_globals)]
pub const XenbusStateInitWait: xenbus_state = xenbus_state(2);
#[allow(non_upper_case_globals)]
pub const XenbusStateInitialised: xenbus_state = xenbus_state(3);
#[allow(non_upper_case_globals)]
pub const XenbusStateConnected: xenbus_state = xenbus_state(4);
#[allow(non_upper_case_globals)]
pub const XenbusStateClosing: xenbus_state = xenbus_state(5);
#[allow(non_upper_case_globals)]
pub const XenbusStateClosed: xenbus_state = xenbus_state(6);
#[allow(non_upper_case_globals)]
pub const XenbusStateReconfiguring: xenbus_state = xenbus_state(7);
#[allow(non_upper_case_globals)]
pub const XenbusStateReconfigured: xenbus_state = xenbus_state(8);
