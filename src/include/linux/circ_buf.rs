// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/circ_buf.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013591

/// C layout of `struct circ_buf`.
///
/// The frozen kernel commands use `-funsigned-char`; consequently `buf`
/// points at unsigned octets.  The buffer allocation, ownership, and all
/// synchronization remain with its C-equivalent caller.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct circ_buf {
    pub buf: *mut u8,
    pub head: core::ffi::c_int,
    pub tail: core::ffi::c_int,
}

/* Return count in buffer. */
//
// This intentionally remains an expression macro: the upstream definition is
// polymorphic and the selected ring-buffer caller supplies `unsigned long`.
// Bind each operand once, then use target-width wrapping operations so the
// arithmetic retains the kernel machine-integer behavior in every Rust build
// mode.  Valid circular-buffer indices have a power-of-two `size`.
#[macro_export]
macro_rules! CIRC_CNT {
    ($head:expr, $tail:expr, $size:expr) => {{
        let __circ_head = $head;
        let __circ_tail = $tail;
        let __circ_size = $size;
        __circ_head.wrapping_sub(__circ_tail) & __circ_size.wrapping_sub(1)
    }};
}

/* Return space available, 0..size-1.  We always leave one free char
 * as a completely full buffer has head == tail, which is the same as
 * empty. */
#[macro_export]
macro_rules! CIRC_SPACE {
    ($head:expr, $tail:expr, $size:expr) => {{
        let __circ_head = $head;
        let __circ_tail = $tail;
        let __circ_size = $size;
        $crate::CIRC_CNT!(
            __circ_tail,
            __circ_head.wrapping_add(1),
            __circ_size,
        )
    }};
}

/* Return count up to the end of the buffer.  Carefully avoid
 * accessing head and tail more than once, so they can change
 * underneath us without returning inconsistent results. */
//
// Upstream's GNU statement expression stores both temporaries as C `int`.
// The casts therefore occur after the same target-width counter arithmetic;
// casting `end` back to the caller's counter type models C's usual arithmetic
// conversion when `head` is an unsigned long.
#[macro_export]
macro_rules! CIRC_CNT_TO_END {
    ($head:expr, $tail:expr, $size:expr) => {{
        let __circ_head = $head;
        let __circ_tail = $tail;
        let __circ_size = $size;
        let end = __circ_size.wrapping_sub(__circ_tail) as core::ffi::c_int;
        let n = (__circ_head.wrapping_add(end as _)
            & __circ_size.wrapping_sub(1)) as core::ffi::c_int;
        if n < end { n } else { end }
    }};
}

/* Return space available up to the end of the buffer. */
#[macro_export]
macro_rules! CIRC_SPACE_TO_END {
    ($head:expr, $tail:expr, $size:expr) => {{
        let __circ_head = $head;
        let __circ_tail = $tail;
        let __circ_size = $size;
        let end = __circ_size
            .wrapping_sub(1)
            .wrapping_sub(__circ_head) as core::ffi::c_int;
        let n = ((end as _).wrapping_add(__circ_tail)
            & __circ_size.wrapping_sub(1)) as core::ffi::c_int;
        if n <= end { n } else { end.wrapping_add(1) }
    }};
}
