// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/sunrpc/gss_err.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S015088

/*
 * Adapted from MIT Kerberos 5-1.2.1 include/gssapi/gssapi.h.
 *
 * Copyright (c) 2002 The Regents of the University of Michigan.
 * All rights reserved.
 */

/*
 * Copyright 1993 by OpenVision Technologies, Inc.
 *
 * Permission to use, copy, modify, distribute, and sell this software
 * and its documentation for any purpose is hereby granted without fee,
 * provided that the above copyright notice appears in all copies and
 * that both that copyright notice and this permission notice appear in
 * supporting documentation, and that the name of OpenVision not be used
 * in advertising or publicity pertaining to distribution of the software
 * without specific, written prior permission. OpenVision makes no
 * representations about the suitability of this software for any
 * purpose.  It is provided "as is" without express or implied warranty.
 *
 * OPENVISION DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE,
 * INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS, IN NO
 * EVENT SHALL OPENVISION BE LIABLE FOR ANY SPECIAL, INDIRECT OR
 * CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF
 * USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
 * OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
 * PERFORMANCE OF THIS SOFTWARE.
 */

pub type OM_uint32 = u32;

pub const GSS_C_DELEG_FLAG: i32 = 1;
pub const GSS_C_MUTUAL_FLAG: i32 = 2;
pub const GSS_C_REPLAY_FLAG: i32 = 4;
pub const GSS_C_SEQUENCE_FLAG: i32 = 8;
pub const GSS_C_CONF_FLAG: i32 = 16;
pub const GSS_C_INTEG_FLAG: i32 = 32;
pub const GSS_C_ANON_FLAG: i32 = 64;
pub const GSS_C_PROT_READY_FLAG: i32 = 128;
pub const GSS_C_TRANS_FLAG: i32 = 256;

pub const GSS_C_BOTH: i32 = 0;
pub const GSS_C_INITIATE: i32 = 1;
pub const GSS_C_ACCEPT: i32 = 2;

pub const GSS_C_GSS_CODE: i32 = 1;
pub const GSS_C_MECH_CODE: i32 = 2;

pub const GSS_C_INDEFINITE: OM_uint32 = 0xffff_ffff;

pub const GSS_S_COMPLETE: i32 = 0;

pub const GSS_C_CALLING_ERROR_OFFSET: i32 = 24;
pub const GSS_C_ROUTINE_ERROR_OFFSET: i32 = 16;
pub const GSS_C_SUPPLEMENTARY_OFFSET: i32 = 0;
pub const GSS_C_CALLING_ERROR_MASK: OM_uint32 = 0o377;
pub const GSS_C_ROUTINE_ERROR_MASK: OM_uint32 = 0o377;
pub const GSS_C_SUPPLEMENTARY_MASK: OM_uint32 = 0o177777;

mod private {
    pub trait Sealed {}
}

/// The C integer category and result category of the function-like GSS macros.
///
/// `gss_err.h` applies the C usual arithmetic conversions to its uncast macro
/// argument.  The implementations below retain the resulting category instead
/// of narrowing every input to `OM_uint32`.
pub trait GssStatusCode: private::Sealed {
    type CResult;

    fn gss_calling_error(self) -> Self::CResult;
    fn gss_routine_error(self) -> Self::CResult;
    fn gss_supplementary_info(self) -> Self::CResult;
    fn gss_error(self) -> Self::CResult;
    fn gss_calling_error_field(self) -> Self::CResult;
    fn gss_routine_error_field(self) -> Self::CResult;
    fn gss_supplementary_info_field(self) -> Self::CResult;
}

macro_rules! impl_gss_status_code_u32 {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl GssStatusCode for $ty {
                type CResult = OM_uint32;

                #[inline]
                fn gss_calling_error(self) -> Self::CResult {
                    let x = self as OM_uint32;
                    x & (GSS_C_CALLING_ERROR_MASK << GSS_C_CALLING_ERROR_OFFSET)
                }

                #[inline]
                fn gss_routine_error(self) -> Self::CResult {
                    let x = self as OM_uint32;
                    x & (GSS_C_ROUTINE_ERROR_MASK << GSS_C_ROUTINE_ERROR_OFFSET)
                }

                #[inline]
                fn gss_supplementary_info(self) -> Self::CResult {
                    let x = self as OM_uint32;
                    x & (GSS_C_SUPPLEMENTARY_MASK << GSS_C_SUPPLEMENTARY_OFFSET)
                }

                #[inline]
                fn gss_error(self) -> Self::CResult {
                    let x = self as OM_uint32;
                    x & ((GSS_C_CALLING_ERROR_MASK << GSS_C_CALLING_ERROR_OFFSET)
                        | (GSS_C_ROUTINE_ERROR_MASK << GSS_C_ROUTINE_ERROR_OFFSET))
                }

                #[inline]
                fn gss_calling_error_field(self) -> Self::CResult {
                    let x = self as OM_uint32;
                    (x >> GSS_C_CALLING_ERROR_OFFSET) & GSS_C_CALLING_ERROR_MASK
                }

                #[inline]
                fn gss_routine_error_field(self) -> Self::CResult {
                    let x = self as OM_uint32;
                    (x >> GSS_C_ROUTINE_ERROR_OFFSET) & GSS_C_ROUTINE_ERROR_MASK
                }

                #[inline]
                fn gss_supplementary_info_field(self) -> Self::CResult {
                    let x = self as OM_uint32;
                    (x >> GSS_C_SUPPLEMENTARY_OFFSET) & GSS_C_SUPPLEMENTARY_MASK
                }
            }
        )+
    };
}

macro_rules! impl_gss_status_code_identity {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl GssStatusCode for $ty {
                type CResult = $ty;

                #[inline]
                fn gss_calling_error(self) -> Self::CResult {
                    self & ((GSS_C_CALLING_ERROR_MASK as $ty) << GSS_C_CALLING_ERROR_OFFSET)
                }

                #[inline]
                fn gss_routine_error(self) -> Self::CResult {
                    self & ((GSS_C_ROUTINE_ERROR_MASK as $ty) << GSS_C_ROUTINE_ERROR_OFFSET)
                }

                #[inline]
                fn gss_supplementary_info(self) -> Self::CResult {
                    self & ((GSS_C_SUPPLEMENTARY_MASK as $ty) << GSS_C_SUPPLEMENTARY_OFFSET)
                }

                #[inline]
                fn gss_error(self) -> Self::CResult {
                    self & (((GSS_C_CALLING_ERROR_MASK as $ty) << GSS_C_CALLING_ERROR_OFFSET)
                        | ((GSS_C_ROUTINE_ERROR_MASK as $ty) << GSS_C_ROUTINE_ERROR_OFFSET))
                }

                #[inline]
                fn gss_calling_error_field(self) -> Self::CResult {
                    (self >> GSS_C_CALLING_ERROR_OFFSET) & (GSS_C_CALLING_ERROR_MASK as $ty)
                }

                #[inline]
                fn gss_routine_error_field(self) -> Self::CResult {
                    (self >> GSS_C_ROUTINE_ERROR_OFFSET) & (GSS_C_ROUTINE_ERROR_MASK as $ty)
                }

                #[inline]
                fn gss_supplementary_info_field(self) -> Self::CResult {
                    (self >> GSS_C_SUPPLEMENTARY_OFFSET) & (GSS_C_SUPPLEMENTARY_MASK as $ty)
                }
            }
        )+
    };
}

// C promotes these categories before combining them with an `unsigned int`
// mask.  The result is consequently `OM_uint32` on both frozen LP64 targets.
impl_gss_status_code_u32!(bool, i8, u8, i16, u16, i32, OM_uint32);

// A signed type wider than `unsigned int` can represent every `OM_uint32`
// value; an unsigned wider type remains unsigned.  Thus C leaves these
// categories as the result category of the bitwise operation.
impl_gss_status_code_identity!(i64, u64, isize, usize, i128, u128);

#[allow(non_snake_case)]
#[inline]
pub fn GSS_CALLING_ERROR<T: GssStatusCode>(x: T) -> T::CResult {
    x.gss_calling_error()
}

#[allow(non_snake_case)]
#[inline]
pub fn GSS_ROUTINE_ERROR<T: GssStatusCode>(x: T) -> T::CResult {
    x.gss_routine_error()
}

#[allow(non_snake_case)]
#[inline]
pub fn GSS_SUPPLEMENTARY_INFO<T: GssStatusCode>(x: T) -> T::CResult {
    x.gss_supplementary_info()
}

#[allow(non_snake_case)]
#[inline]
pub fn GSS_ERROR<T: GssStatusCode>(x: T) -> T::CResult {
    x.gss_error()
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

pub const GSS_S_CONTINUE_NEEDED: i32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 0);
pub const GSS_S_DUPLICATE_TOKEN: i32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 1);
pub const GSS_S_OLD_TOKEN: i32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 2);
pub const GSS_S_UNSEQ_TOKEN: i32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 3);
pub const GSS_S_GAP_TOKEN: i32 = 1 << (GSS_C_SUPPLEMENTARY_OFFSET + 4);

#[allow(non_snake_case)]
#[inline]
pub fn GSS_CALLING_ERROR_FIELD<T: GssStatusCode>(x: T) -> T::CResult {
    x.gss_calling_error_field()
}

#[allow(non_snake_case)]
#[inline]
pub fn GSS_ROUTINE_ERROR_FIELD<T: GssStatusCode>(x: T) -> T::CResult {
    x.gss_routine_error_field()
}

#[allow(non_snake_case)]
#[inline]
pub fn GSS_SUPPLEMENTARY_INFO_FIELD<T: GssStatusCode>(x: T) -> T::CResult {
    x.gss_supplementary_info_field()
}

pub const GSS_S_CRED_UNAVAIL: OM_uint32 = GSS_S_FAILURE;
