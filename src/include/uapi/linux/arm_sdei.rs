// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/arm_sdei.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016053

// Copyright (C) 2017 Arm Ltd.

/// Base SMCCC function number for SDEI v1.0 services.
pub const SDEI_1_0_FN_BASE: u32 = 0xC400_0020;
pub const SDEI_1_0_MASK: u32 = 0xFFFF_FFE0;

mod private {
    pub trait Sealed {}
}

/// C integer-promotion and usual-arithmetic-conversion mapping for
/// `SDEI_1_0_FN(n)` on the frozen AArch64 LP64 ABI.
///
/// The macro's base literal is `unsigned int`.  Narrow operands first promote
/// to `int` and then convert to `unsigned int`; a 64-bit signed operand keeps
/// its signed category because it represents every `unsigned int` value; and
/// a 64-bit unsigned operand remains unsigned.
pub trait Sdei1_0FnInput: private::Sealed {
    type Output;

    fn sdei_1_0_fn(self) -> Self::Output;
}

/// C integer-promotion and usual-arithmetic-conversion mapping for the two
/// version macros whose masks are `int` literals.
pub trait SdeiVersionIntMaskInput: private::Sealed {
    type Output;

    fn sdei_version_major(self) -> Self::Output;
    fn sdei_version_minor(self) -> Self::Output;
}

/// C integer-promotion and usual-arithmetic-conversion mapping for
/// `SDEI_VERSION_VENDOR`, whose `0xffffffff` mask is `unsigned int`.
pub trait SdeiVersionVendorMaskInput: private::Sealed {
    type Output;

    fn sdei_version_vendor(self) -> Self::Output;
}

macro_rules! impl_sdei_promoted_integer_input {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl Sdei1_0FnInput for $ty {
                type Output = u32;

                #[inline]
                fn sdei_1_0_fn(self) -> Self::Output {
                    SDEI_1_0_FN_BASE.wrapping_add(self as u32)
                }
            }

            impl SdeiVersionIntMaskInput for $ty {
                type Output = i32;

                #[inline]
                fn sdei_version_major(self) -> Self::Output {
                    (self as i32).wrapping_shr(SDEI_VERSION_MAJOR_SHIFT as u32)
                        & SDEI_VERSION_MAJOR_MASK
                }

                #[inline]
                fn sdei_version_minor(self) -> Self::Output {
                    (self as i32).wrapping_shr(SDEI_VERSION_MINOR_SHIFT as u32)
                        & SDEI_VERSION_MINOR_MASK
                }
            }

            impl SdeiVersionVendorMaskInput for $ty {
                type Output = u32;

                #[inline]
                fn sdei_version_vendor(self) -> Self::Output {
                    (self as u32) & SDEI_VERSION_VENDOR_MASK
                }
            }
        )+
    };
}

// C promotes these categories to `int` before the macro operations.
impl_sdei_promoted_integer_input!(bool, i8, u8, i16, u16, i32);

impl private::Sealed for u32 {}

impl Sdei1_0FnInput for u32 {
    type Output = u32;

    #[inline]
    fn sdei_1_0_fn(self) -> Self::Output {
        SDEI_1_0_FN_BASE.wrapping_add(self)
    }
}

impl SdeiVersionIntMaskInput for u32 {
    type Output = u32;

    #[inline]
    fn sdei_version_major(self) -> Self::Output {
        self.wrapping_shr(SDEI_VERSION_MAJOR_SHIFT as u32)
            & (SDEI_VERSION_MAJOR_MASK as u32)
    }

    #[inline]
    fn sdei_version_minor(self) -> Self::Output {
        self.wrapping_shr(SDEI_VERSION_MINOR_SHIFT as u32)
            & (SDEI_VERSION_MINOR_MASK as u32)
    }
}

impl SdeiVersionVendorMaskInput for u32 {
    type Output = u32;

    #[inline]
    fn sdei_version_vendor(self) -> Self::Output {
        self & SDEI_VERSION_VENDOR_MASK
    }
}

macro_rules! impl_sdei_wide_signed_input {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl Sdei1_0FnInput for $ty {
                type Output = $ty;

                #[inline]
                fn sdei_1_0_fn(self) -> Self::Output {
                    (SDEI_1_0_FN_BASE as $ty).wrapping_add(self)
                }
            }

            impl SdeiVersionIntMaskInput for $ty {
                type Output = $ty;

                #[inline]
                fn sdei_version_major(self) -> Self::Output {
                    (self >> SDEI_VERSION_MAJOR_SHIFT) & (SDEI_VERSION_MAJOR_MASK as $ty)
                }

                #[inline]
                fn sdei_version_minor(self) -> Self::Output {
                    (self >> SDEI_VERSION_MINOR_SHIFT) & (SDEI_VERSION_MINOR_MASK as $ty)
                }
            }

            impl SdeiVersionVendorMaskInput for $ty {
                type Output = $ty;

                #[inline]
                fn sdei_version_vendor(self) -> Self::Output {
                    self & (SDEI_VERSION_VENDOR_MASK as $ty)
                }
            }
        )+
    };
}

// On frozen AArch64 LP64, a signed 64-bit (or wider) operand can represent
// every `unsigned int` value, so the usual arithmetic conversions retain it.
impl_sdei_wide_signed_input!(i64, isize, i128);

macro_rules! impl_sdei_wide_unsigned_input {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl Sdei1_0FnInput for $ty {
                type Output = $ty;

                #[inline]
                fn sdei_1_0_fn(self) -> Self::Output {
                    (SDEI_1_0_FN_BASE as $ty).wrapping_add(self)
                }
            }

            impl SdeiVersionIntMaskInput for $ty {
                type Output = $ty;

                #[inline]
                fn sdei_version_major(self) -> Self::Output {
                    (self >> SDEI_VERSION_MAJOR_SHIFT) & (SDEI_VERSION_MAJOR_MASK as $ty)
                }

                #[inline]
                fn sdei_version_minor(self) -> Self::Output {
                    (self >> SDEI_VERSION_MINOR_SHIFT) & (SDEI_VERSION_MINOR_MASK as $ty)
                }
            }

            impl SdeiVersionVendorMaskInput for $ty {
                type Output = $ty;

                #[inline]
                fn sdei_version_vendor(self) -> Self::Output {
                    self & (SDEI_VERSION_VENDOR_MASK as $ty)
                }
            }
        )+
    };
}

// The usual arithmetic conversions keep wider unsigned operands unsigned.
impl_sdei_wide_unsigned_input!(u64, usize, u128);

/// C `SDEI_1_0_FN(n)`, including the operand's usual arithmetic conversions.
#[allow(non_snake_case)]
#[inline]
pub fn SDEI_1_0_FN<T: Sdei1_0FnInput>(n: T) -> T::Output {
    n.sdei_1_0_fn()
}

// These are the header's `SDEI_1_0_FN(<int literal>)` expansions.  Keep the
// literals explicitly `i32` before the source macro's conversion to `u32`.
pub const SDEI_1_0_FN_SDEI_VERSION: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x00_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_REGISTER: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x01_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_ENABLE: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x02_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_DISABLE: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x03_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_CONTEXT: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x04_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_COMPLETE: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x05_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_COMPLETE_AND_RESUME: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x06_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_UNREGISTER: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x07_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_STATUS: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x08_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_GET_INFO: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x09_i32 as u32);
pub const SDEI_1_0_FN_SDEI_EVENT_ROUTING_SET: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x0A_i32 as u32);
pub const SDEI_1_0_FN_SDEI_PE_MASK: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x0B_i32 as u32);
pub const SDEI_1_0_FN_SDEI_PE_UNMASK: u32 = SDEI_1_0_FN_BASE.wrapping_add(0x0C_i32 as u32);
pub const SDEI_1_0_FN_SDEI_INTERRUPT_BIND: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x0D_i32 as u32);
pub const SDEI_1_0_FN_SDEI_INTERRUPT_RELEASE: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x0E_i32 as u32);
pub const SDEI_1_0_FN_SDEI_PRIVATE_RESET: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x11_i32 as u32);
pub const SDEI_1_0_FN_SDEI_SHARED_RESET: u32 =
    SDEI_1_0_FN_BASE.wrapping_add(0x12_i32 as u32);

pub const SDEI_VERSION_MAJOR_SHIFT: i32 = 48;
pub const SDEI_VERSION_MAJOR_MASK: i32 = 0x7fff;
pub const SDEI_VERSION_MINOR_SHIFT: i32 = 32;
pub const SDEI_VERSION_MINOR_MASK: i32 = 0xffff;
pub const SDEI_VERSION_VENDOR_SHIFT: i32 = 0;
pub const SDEI_VERSION_VENDOR_MASK: u32 = 0xffff_ffff;

/// C `SDEI_VERSION_MAJOR(x)`, including integer promotions and the usual
/// arithmetic conversions with its `int` mask.
#[allow(non_snake_case)]
#[inline]
pub fn SDEI_VERSION_MAJOR<T: SdeiVersionIntMaskInput>(x: T) -> T::Output {
    x.sdei_version_major()
}

/// C `SDEI_VERSION_MINOR(x)`, including integer promotions and the usual
/// arithmetic conversions with its `int` mask.
#[allow(non_snake_case)]
#[inline]
pub fn SDEI_VERSION_MINOR<T: SdeiVersionIntMaskInput>(x: T) -> T::Output {
    x.sdei_version_minor()
}

/// C `SDEI_VERSION_VENDOR(x)`, including integer promotions and the usual
/// arithmetic conversions with its `unsigned int` mask.
#[allow(non_snake_case)]
#[inline]
pub fn SDEI_VERSION_VENDOR<T: SdeiVersionVendorMaskInput>(x: T) -> T::Output {
    x.sdei_version_vendor()
}

// SDEI return values.
pub const SDEI_SUCCESS: i32 = 0;
pub const SDEI_NOT_SUPPORTED: i32 = -1;
pub const SDEI_INVALID_PARAMETERS: i32 = -2;
pub const SDEI_DENIED: i32 = -3;
pub const SDEI_PENDING: i32 = -5;
pub const SDEI_OUT_OF_RESOURCE: i32 = -10;

// EVENT_REGISTER flags.
pub const SDEI_EVENT_REGISTER_RM_ANY: i32 = 0;
pub const SDEI_EVENT_REGISTER_RM_PE: i32 = 1;

// EVENT_STATUS return value bits.
pub const SDEI_EVENT_STATUS_RUNNING: i32 = 2;
pub const SDEI_EVENT_STATUS_ENABLED: i32 = 1;
pub const SDEI_EVENT_STATUS_REGISTERED: i32 = 0;

// EVENT_COMPLETE status values.
pub const SDEI_EV_HANDLED: i32 = 0;
pub const SDEI_EV_FAILED: i32 = 1;

// GET_INFO values and their results.
pub const SDEI_EVENT_INFO_EV_TYPE: i32 = 0;
pub const SDEI_EVENT_INFO_EV_SIGNALED: i32 = 1;
pub const SDEI_EVENT_INFO_EV_PRIORITY: i32 = 2;
pub const SDEI_EVENT_INFO_EV_ROUTING_MODE: i32 = 3;
pub const SDEI_EVENT_INFO_EV_ROUTING_AFF: i32 = 4;

pub const SDEI_EVENT_TYPE_PRIVATE: i32 = 0;
pub const SDEI_EVENT_TYPE_SHARED: i32 = 1;
pub const SDEI_EVENT_PRIORITY_NORMAL: i32 = 0;
pub const SDEI_EVENT_PRIORITY_CRITICAL: i32 = 1;
