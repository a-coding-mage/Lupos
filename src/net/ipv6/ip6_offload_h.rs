// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: net/ipv6/ip6_offload.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S017908

//! IPv6 GSO/GRO offload initialization and teardown declarations.

use core::ffi::c_int;

unsafe extern "C" {
    pub fn ipv6_exthdrs_offload_init() -> c_int;
    pub fn udpv6_offload_init() -> c_int;
    pub fn udpv6_offload_exit() -> c_int;
    pub fn tcpv6_offload_init() -> c_int;
}
