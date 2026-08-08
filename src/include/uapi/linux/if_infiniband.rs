/* SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-2-Clause) */
/*
 * This software is available to you under a choice of one of two
 * licenses.  You may choose to be licensed under the terms of the GNU
 * General Public License (GPL) Version 2, available at
 * <http://www.fsf.org/copyleft/gpl.html>, or the OpenIB.org BSD
 * license, available in the LICENSE.TXT file accompanying this
 * software.  These details are also available at
 * <http://www.openfabrics.org/software_license.htm>.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
 * BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
 * ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 * Copyright (c) 2004 Topspin Communications.  All rights reserved.
 *
 * $Id$
 */
//! linux-source: include/uapi/linux/if_infiniband.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016168

// The C include guard `_LINUX_IF_INFINIBAND_H` is provided by Rust's module
// boundary; there is no C preprocessor guard to bridge into this namespace.

/// Octets in an IPoIB hardware address.
///
/// The Linux macro expands to the C integer literal `20`; `i32` preserves the
/// literal's C `int` value domain for expressions using this UAPI constant.
pub const INFINIBAND_ALEN: i32 = 20;
