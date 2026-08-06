// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/reboot.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016353

/*
 * Magic values required to use the `_reboot()` system call.
 *
 * The types preserve the C types of the unsuffixed integer literal macros:
 * values not representable by C `int` are `unsigned int` (`u32`), while the
 * remaining values are C `int` (`i32`).
 */
pub const LINUX_REBOOT_MAGIC1: u32 = 0xfee1dead;
pub const LINUX_REBOOT_MAGIC2: i32 = 672274793;
pub const LINUX_REBOOT_MAGIC2A: i32 = 85072278;
pub const LINUX_REBOOT_MAGIC2B: i32 = 369367448;
pub const LINUX_REBOOT_MAGIC2C: i32 = 537993216;

/*
 * Commands accepted by the `_reboot()` system call.
 *
 * RESTART     Restart system using default command and mode.
 * HALT        Stop OS and give system control to ROM monitor, if any.
 * CAD_ON      Ctrl-Alt-Del sequence causes RESTART command.
 * CAD_OFF     Ctrl-Alt-Del sequence sends SIGINT to init task.
 * POWER_OFF   Stop OS and remove all power from system, if possible.
 * RESTART2    Restart system using given command string.
 * SW_SUSPEND  Suspend system using software suspend if compiled in.
 * KEXEC       Restart system using a previously loaded Linux kernel.
 */
pub const LINUX_REBOOT_CMD_RESTART: i32 = 0x01234567;
pub const LINUX_REBOOT_CMD_HALT: u32 = 0xcdef0123;
pub const LINUX_REBOOT_CMD_CAD_ON: u32 = 0x89abcdef;
pub const LINUX_REBOOT_CMD_CAD_OFF: i32 = 0x00000000;
pub const LINUX_REBOOT_CMD_POWER_OFF: u32 = 0x4321fedc;
pub const LINUX_REBOOT_CMD_RESTART2: u32 = 0xa1b2c3d4;
pub const LINUX_REBOOT_CMD_SW_SUSPEND: u32 = 0xd000fce2;
pub const LINUX_REBOOT_CMD_KEXEC: i32 = 0x45584543;
