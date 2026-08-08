// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/sunrpc/gss_err.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S015088

/*
 * Adapted from MIT Kerberos 5-1.2.1 include/gssapi/gssapi.h
 *
 * Copyright (c) 2002 The Regents of the University of Michigan.
 * All rights reserved.
 *
 * Andy Adamson <andros@umich.edu>
 */

/*
 * Copyright 1993 by OpenVision Technologies, Inc.
 *
 * Permission to use, copy, modify, distribute, and sell this software and
 * its documentation for any purpose is hereby granted without fee, provided
 * that the above copyright notice appears in all copies and that both that
 * copyright notice and this permission notice appear in supporting
 * documentation, and that the name of OpenVision not be used in advertising
 * or publicity pertaining to distribution of the software without specific,
 * written prior permission. OpenVision makes no representations about the
 * suitability of this software for any purpose. It is provided "as is"
 * without express or implied warranty.
 *
 * OPENVISION DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE, INCLUDING
 * ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS, IN NO EVENT SHALL
 * OPENVISION BE LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR
 * ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER
 * IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT
 * OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

pub type OM_uint32 = u32;

pub const GSS_C_DELEG_FLAG: OM_uint32 = 1;
pub const GSS_C_MUTUAL_FLAG: OM_uint32 = 2;
pub const GSS_C_REPLAY_FLAG: OM_uint32 = 4;
pub const GSS_C_SEQUENCE_FLAG: OM_uint32 = 8;
pub const GSS_C_CONF_FLAG: OM_uint32 = 16;
pub const GSS_C_INTEG_FLAG: OM_uint32 = 32;
pub const GSS_C_ANON_FLAG: OM_uint32 = 64;
pub const GSS_C_PROT_READY_FLAG: OM_uint32 = 128;
pub const GSS_C_TRANS_FLAG: OM_uint32 = 256;

pub const GSS_C_BOTH: OM_uint32 = 0;
pub const GSS_C_INITIATE: OM_uint32 = 1;
pub const GSS_C_ACCEPT: OM_uint32 = 2;

pub const GSS_C_GSS_CODE: OM_uint32 = 1;
pub const GSS_C_MECH_CODE: OM_uint32 = 2;

pub const GSS_C_INDEFINITE: OM_uint32 = 0xffff_ffff;

pub const GSS_S_COMPLETE: OM_uint32 = 0;

pub const GSS_C_CALLING_ERROR_OFFSET: OM_uint32 = 24;
pub const GSS_C_ROUTINE_ERROR_OFFSET: OM_uint32 = 16;
pub const GSS_C_SUPPLEMENTARY_OFFSET: OM_uint32 = 0;
pub const GSS_C_CALLING_ERROR_MASK: OM_uint32 = 0o377;
pub const GSS_C_ROUTINE_ERROR_MASK: OM_uint32 = 0o377;
pub const GSS_C_SUPPLEMENTARY_MASK: OM_uint32 = 0o177_777;

#[macro_export]
macro_rules! GSS_CALLING_ERROR {
    ($x:expr) => {
        ($x) & (0o377u32 << 24u32)
    };
}

#[macro_export]
macro_rules! GSS_ROUTINE_ERROR {
    ($x:expr) => {
        ($x) & (0o377u32 << 16u32)
    };
}

#[macro_export]
macro_rules! GSS_SUPPLEMENTARY_INFO {
    ($x:expr) => {
        ($x) & (0o177_777u32 << 0u32)
    };
}

#[macro_export]
macro_rules! GSS_ERROR {
    ($x:expr) => {
        ($x) & ((0o377u32 << 24u32) | (0o377u32 << 16u32))
    };
}

pub const GSS_S_CALL_INACCESSIBLE_READ: OM_uint32 = 1 << GSS_C_CALLING_ERROR_OFFSET;
pub const GSS_S_CALL_INACCESSIBLE_WRITE: OM_uint32 = 2 << GSS_C_CALLING_ERROR_OFFSET;
pub const GSS_S_CALL_BAD_STRUCTURE: OM_uint32 = 3 << GSS_C_CALLING_ERROR_OFFSET;

pub const GSS_S_BAD_MECH: OM_uint32 = 1 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_BAD_NAME: OM_uint32 = 2 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_BAD_NAMETYPE: OM_uint32 = 3 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_BAD_BINDINGS: OM_uint32 = 4 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_BAD_STATUS: OM_uint32 = 5 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_BAD_SIG: OM_uint32 = 6 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_NO_CRED: OM_uint32 = 7 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_NO_CONTEXT: OM_uint32 = 8 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_DEFECTIVE_TOKEN: OM_uint32 = 9 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_DEFECTIVE_CREDENTIAL: OM_uint32 = 10 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_CREDENTIALS_EXPIRED: OM_uint32 = 11 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_CONTEXT_EXPIRED: OM_uint32 = 12 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_FAILURE: OM_uint32 = 13 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_BAD_QOP: OM_uint32 = 14 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_UNAUTHORIZED: OM_uint32 = 15 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_UNAVAILABLE: OM_uint32 = 16 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_DUPLICATE_ELEMENT: OM_uint32 = 17 << GSS_C_ROUTINE_ERROR_OFFSET;
pub const GSS_S_NAME_NOT_MN: OM_uint32 = 18 << GSS_C_ROUTINE_ERROR_OFFSET;

pub const GSS_S_CONTINUE_NEEDED: OM_uint32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 0);
pub const GSS_S_DUPLICATE_TOKEN: OM_uint32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 1);
pub const GSS_S_OLD_TOKEN: OM_uint32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 2);
pub const GSS_S_UNSEQ_TOKEN: OM_uint32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 3);
pub const GSS_S_GAP_TOKEN: OM_uint32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 4);

#[macro_export]
macro_rules! GSS_CALLING_ERROR_FIELD {
    ($x:expr) => {
        (($x) >> 24u32) & 0o377u32
    };
}

#[macro_export]
macro_rules! GSS_ROUTINE_ERROR_FIELD {
    ($x:expr) => {
        (($x) >> 16u32) & 0o377u32
    };
}

#[macro_export]
macro_rules! GSS_SUPPLEMENTARY_INFO_FIELD {
    ($x:expr) => {
        (($x) >> 0u32) & 0o177_777u32
    };
}

pub const GSS_S_CRED_UNAVAIL: OM_uint32 = GSS_S_FAILURE;
