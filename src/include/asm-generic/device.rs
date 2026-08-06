// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/asm-generic/device.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S012533

/*
 * Arch specific extensions to struct device.
 *
 * The generic Linux definitions intentionally have no members.  `repr(C)`
 * retains their C aggregate role when embedded by value in device layouts.
 */
#[repr(C)]
pub struct dev_archdata {}

#[repr(C)]
pub struct pdev_archdata {}
