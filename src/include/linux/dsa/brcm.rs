// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/dsa/brcm.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S013804

// Copyright (C) 2014 Broadcom Corporation

/* Broadcom tag specific helpers to insert and extract queue/port number. */
//
// These remain expression macros, as in the source header.  The selected
// producer supplies an unsigned-int port index and a `u16` queue mapping, so
// C's usual arithmetic conversions make the source expression unsigned-int.
// The selected consumer supplies a `u16` mapping; its C integer promotion is
// non-negative and the extracted fields are assigned to unsigned-int locals.
// Each macro evaluates its argument exactly once.
#[macro_export]
macro_rules! BRCM_TAG_SET_PORT_QUEUE {
    ($p:expr, $($q:tt)+) => {{
        struct BrcmTagPortQueue(u32);

        impl ::core::ops::BitOr<u16> for BrcmTagPortQueue {
            type Output = u32;

            fn bitor(self, rhs: u16) -> Self::Output {
                self.0 | (rhs as u32)
            }
        }

        // `q` is deliberately transcribed without parentheses, matching the
        // source macro replacement list. The selected `u16` right operand is
        // converted at the `|`, as by C's usual arithmetic conversions.
        BrcmTagPortQueue((($p) as u32).wrapping_shl(8)) | $($q)+
    }};
}

#[macro_export]
macro_rules! BRCM_TAG_GET_PORT {
    ($v:expr) => {
        ((($v) as u32) >> 8)
    };
}

#[macro_export]
macro_rules! BRCM_TAG_GET_QUEUE {
    ($v:expr) => {
        ((($v) as u32) & 0xff)
    };
}
