// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: arch/arm64/include/uapi/asm/auxvec.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S000209

/*
 * Copyright (C) 2012 ARM Ltd.
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

/// Auxiliary-vector key for the ELF vDSO base address.
pub const AT_SYSINFO_EHDR: i32 = 33;

/// Auxiliary-vector key for the signal-delivery stack size.
pub const AT_MINSIGSTKSZ: i32 = 51;

/// Number of auxiliary-vector entries emitted by ARM64 `ARCH_DLINFO`.
pub const AT_VECTOR_SIZE_ARCH: i32 = 2;
