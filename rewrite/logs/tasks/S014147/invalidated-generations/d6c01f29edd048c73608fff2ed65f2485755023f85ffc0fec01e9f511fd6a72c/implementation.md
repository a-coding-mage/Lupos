# S014147 implementation record

Oracle: `vendor/linux/include/linux/irqreturn.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header is unconditional for both frozen configurations.  It contributes
the `enum irqreturn` enumerators, the `irqreturn_t` typedef, and `IRQ_RETVAL`.
The selected IRQ-core context (`kernel/irq/handle.c`) accumulates handler
results using `retval |= res`; therefore the representation is a C `int`
alias rather than a closed Rust enum, preserving combined result values and
the `irq_handler_t` function-return ABI used by `include/linux/interrupt.h`.

`IRQ_RETVAL` is represented by a single-evaluation `const fn` over the
integer operands used by the selected consumers: it returns `IRQ_HANDLED` for
any nonzero operand and `IRQ_NONE` for zero, exactly as the C conditional
macro.  The source has no copyright notice beyond its SPDX identifier.

No configuration conditional, allocation, ownership, synchronization, or
unsafe operation exists in this header.
