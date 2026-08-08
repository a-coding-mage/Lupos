// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/if_vlan.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016178

/*
 * VLAN        An implementation of 802.1Q VLAN tagging.
 *
 * The C enums have their Linux UAPI (int) representation.  The union is
 * explicitly named because Rust has no anonymous union fields; its layout
 * and members remain those of the source anonymous union.
 */

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum vlan_ioctl_cmds {
    ADD_VLAN_CMD = 0,
    DEL_VLAN_CMD = 1,
    SET_VLAN_INGRESS_PRIORITY_CMD = 2,
    SET_VLAN_EGRESS_PRIORITY_CMD = 3,
    GET_VLAN_INGRESS_PRIORITY_CMD = 4,
    GET_VLAN_EGRESS_PRIORITY_CMD = 5,
    SET_VLAN_NAME_TYPE_CMD = 6,
    SET_VLAN_FLAG_CMD = 7,
    GET_VLAN_REALDEV_NAME_CMD = 8,
    GET_VLAN_VID_CMD = 9,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum vlan_flags {
    VLAN_FLAG_REORDER_HDR = 0x1,
    VLAN_FLAG_GVRP = 0x2,
    VLAN_FLAG_LOOSE_BINDING = 0x4,
    VLAN_FLAG_MVRP = 0x8,
    VLAN_FLAG_BRIDGE_BINDING = 0x10,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum vlan_name_types {
    VLAN_NAME_TYPE_PLUS_VID = 0,
    VLAN_NAME_TYPE_RAW_PLUS_VID = 1,
    VLAN_NAME_TYPE_PLUS_VID_NO_PAD = 2,
    VLAN_NAME_TYPE_RAW_PLUS_VID_NO_PAD = 3,
    VLAN_NAME_TYPE_HIGHEST = 4,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union vlan_ioctl_args_u {
    pub device2: [i8; 24],
    pub VID: i32,
    pub skb_priority: u32,
    pub name_type: u32,
    pub bind_type: u32,
    pub flag: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct vlan_ioctl_args {
    pub cmd: i32,
    pub device1: [i8; 24],
    pub u: vlan_ioctl_args_u,
    pub vlan_qos: i16,
}
