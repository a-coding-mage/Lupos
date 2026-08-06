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

#[allow(non_upper_case_globals)]
impl xenbus_state {
    pub const XenbusStateUnknown: Self = Self(0);
    pub const XenbusStateInitialising: Self = Self(1);
    pub const XenbusStateInitWait: Self = Self(2);
    pub const XenbusStateInitialised: Self = Self(3);
    pub const XenbusStateConnected: Self = Self(4);
    pub const XenbusStateClosing: Self = Self(5);
    pub const XenbusStateClosed: Self = Self(6);
    pub const XenbusStateReconfiguring: Self = Self(7);
    pub const XenbusStateReconfigured: Self = Self(8);
}
