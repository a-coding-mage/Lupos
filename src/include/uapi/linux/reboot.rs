// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/reboot.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016353

/*
 * Magic values required to use the reboot() system call.
 *
 * These constants retain the unsigned 32-bit values of the corresponding
 * Linux UAPI macros.
 */
pub const LINUX_REBOOT_MAGIC1: u32 = 0xfee1dead;
pub const LINUX_REBOOT_MAGIC2: u32 = 672274793;
pub const LINUX_REBOOT_MAGIC2A: u32 = 85072278;
pub const LINUX_REBOOT_MAGIC2B: u32 = 369367448;
pub const LINUX_REBOOT_MAGIC2C: u32 = 537993216;

/* Commands accepted by the reboot() system call. */
pub const LINUX_REBOOT_CMD_RESTART: u32 = 0x01234567;
pub const LINUX_REBOOT_CMD_HALT: u32 = 0xCDEF0123;
pub const LINUX_REBOOT_CMD_CAD_ON: u32 = 0x89ABCDEF;
pub const LINUX_REBOOT_CMD_CAD_OFF: u32 = 0x00000000;
pub const LINUX_REBOOT_CMD_POWER_OFF: u32 = 0x4321FEDC;
pub const LINUX_REBOOT_CMD_RESTART2: u32 = 0xA1B2C3D4;
pub const LINUX_REBOOT_CMD_SW_SUSPEND: u32 = 0xD000FCE2;
pub const LINUX_REBOOT_CMD_KEXEC: u32 = 0x45584543;
