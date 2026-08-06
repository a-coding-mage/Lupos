// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/if_vlan.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016178

/*
 * VLAN		An implementation of 802.1Q VLAN tagging.
 *
 * Authors:	Ben Greear <greearb@candelatech.com>
 *
 *		This program is free software; you can redistribute it and/or
 *		modify it under the terms of the GNU General Public License
 *		as published by the Free Software Foundation; either version
 *		2 of the License, or (at your option) any later version.
 */

use core::ffi::{c_int, c_short, c_uchar, c_uint};

/* VLAN IOCTLs are found in sockios.h */

/* Passed in vlan_ioctl_args structure to determine behaviour. */
pub type vlan_ioctl_cmds = c_int;

pub const ADD_VLAN_CMD: vlan_ioctl_cmds = 0;
pub const DEL_VLAN_CMD: vlan_ioctl_cmds = 1;
pub const SET_VLAN_INGRESS_PRIORITY_CMD: vlan_ioctl_cmds = 2;
pub const SET_VLAN_EGRESS_PRIORITY_CMD: vlan_ioctl_cmds = 3;
pub const GET_VLAN_INGRESS_PRIORITY_CMD: vlan_ioctl_cmds = 4;
pub const GET_VLAN_EGRESS_PRIORITY_CMD: vlan_ioctl_cmds = 5;
pub const SET_VLAN_NAME_TYPE_CMD: vlan_ioctl_cmds = 6;
pub const SET_VLAN_FLAG_CMD: vlan_ioctl_cmds = 7;
/* If this works, you know it's a VLAN device, btw. */
pub const GET_VLAN_REALDEV_NAME_CMD: vlan_ioctl_cmds = 8;
/* Get the VID of this VLAN (specified by name). */
pub const GET_VLAN_VID_CMD: vlan_ioctl_cmds = 9;

pub type vlan_flags = c_int;

pub const VLAN_FLAG_REORDER_HDR: vlan_flags = 0x1;
pub const VLAN_FLAG_GVRP: vlan_flags = 0x2;
pub const VLAN_FLAG_LOOSE_BINDING: vlan_flags = 0x4;
pub const VLAN_FLAG_MVRP: vlan_flags = 0x8;
pub const VLAN_FLAG_BRIDGE_BINDING: vlan_flags = 0x10;

pub type vlan_name_types = c_int;

/* Name will look like: vlan0005. */
pub const VLAN_NAME_TYPE_PLUS_VID: vlan_name_types = 0;
/* Name will look like: eth1.0005. */
pub const VLAN_NAME_TYPE_RAW_PLUS_VID: vlan_name_types = 1;
/* Name will look like: vlan5. */
pub const VLAN_NAME_TYPE_PLUS_VID_NO_PAD: vlan_name_types = 2;
/* Name will look like: eth0.5. */
pub const VLAN_NAME_TYPE_RAW_PLUS_VID_NO_PAD: vlan_name_types = 3;
pub const VLAN_NAME_TYPE_HIGHEST: vlan_name_types = 4;

#[repr(C)]
#[derive(Copy, Clone)]
pub union vlan_ioctl_args_u {
    pub device2: [c_uchar; 24],
    pub VID: c_int,
    pub skb_priority: c_uint,
    pub name_type: c_uint,
    pub bind_type: c_uint,
    /* Matches vlan_dev_priv flags. */
    pub flag: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vlan_ioctl_args {
    /* Should be one of the vlan_ioctl_cmds enum above. */
    pub cmd: c_int,
    pub device1: [c_uchar; 24],
    pub u: vlan_ioctl_args_u,
    pub vlan_qos: c_short,
}
