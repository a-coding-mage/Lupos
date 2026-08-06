// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/irqreturn.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014147

/*
 * The C `enum irqreturn` has the ordinary C `int` representation in both
 * frozen targets.  Its values are also accumulated with bitwise OR by the
 * IRQ core, so a Rust enum would incorrectly make the valid combined value
 * `IRQ_HANDLED | IRQ_WAKE_THREAD` unrepresentable.
 */
pub type irqreturn = core::ffi::c_int;
pub type irqreturn_t = irqreturn;

/// Interrupt was not from this device or was not handled.
pub const IRQ_NONE: irqreturn = 0 << 0;
/// Interrupt was handled by this device.
pub const IRQ_HANDLED: irqreturn = 1 << 0;
/// Handler requests that its handler thread be woken.
pub const IRQ_WAKE_THREAD: irqreturn = 1 << 1;

/// Rust-side truth conversion for the scalar operand of `IRQ_RETVAL`.
///
/// This models C's conditional-expression conversion for the C scalar forms
/// that arise from the translated kernel sources.  The input is passed by
/// value, so `IRQ_RETVAL` evaluates its expression exactly once before this
/// conversion is applied.
pub trait IrqRetvalOperand {
    /// Returns the C conditional-expression truth value of `self`.
    fn irq_retval_is_true(self) -> bool;
}

macro_rules! impl_irq_retval_operand_for_integers {
    ($($type:ty),+ $(,)?) => {
        $(
            impl IrqRetvalOperand for $type {
                #[inline]
                fn irq_retval_is_true(self) -> bool {
                    self != 0
                }
            }
        )+
    };
}

impl_irq_retval_operand_for_integers!(
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
);

impl IrqRetvalOperand for bool {
    #[inline]
    fn irq_retval_is_true(self) -> bool {
        self
    }
}

impl IrqRetvalOperand for f32 {
    #[inline]
    fn irq_retval_is_true(self) -> bool {
        self != 0.0
    }
}

impl IrqRetvalOperand for f64 {
    #[inline]
    fn irq_retval_is_true(self) -> bool {
        self != 0.0
    }
}

impl<T: ?Sized> IrqRetvalOperand for *const T {
    #[inline]
    fn irq_retval_is_true(self) -> bool {
        !self.is_null()
    }
}

impl<T: ?Sized> IrqRetvalOperand for *mut T {
    #[inline]
    fn irq_retval_is_true(self) -> bool {
        !self.is_null()
    }
}

/*
 * `IRQ_RETVAL(x)` evaluates `x` once and applies C scalar truth conversion.
 * The generic argument is evaluated before the function body, preserving the
 * pinned macro's evaluation count without narrowing integers, booleans,
 * floating-point values, or raw pointers to `c_int`.
 */
#[allow(non_snake_case)]
pub fn IRQ_RETVAL<T: IrqRetvalOperand>(x: T) -> irqreturn_t {
    if x.irq_retval_is_true() {
        IRQ_HANDLED
    } else {
        IRQ_NONE
    }
}
