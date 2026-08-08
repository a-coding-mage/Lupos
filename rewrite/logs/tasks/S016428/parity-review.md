# Parity review — S016428 / P02 attempt 1 / slot 1

Scope reviewed: the pinned `vendor/linux/include/uapi/linux/tty_flags.h`,
`src/include/uapi/linux/tty_flags.rs`, the current candidate snapshot, and
the S016428 frozen task/scope/symbol rows. No compiler, formatter, test,
historical source, implementation rationale, or other task evidence was used.

## Finding P1 — `__KERNEL__` conditional API is erased

Linux symbols: `ASYNCB_INITIALIZED`, `ASYNCB_SUSPENDED`,
`ASYNCB_NORMAL_ACTIVE`, `ASYNCB_BOOT_AUTOCONF`, `ASYNCB_CLOSING`,
`ASYNCB_CTS_FLOW`, `ASYNCB_CHECK_CD`, `ASYNCB_SHARE_IRQ`,
`ASYNCB_CONS_FLOW`, `ASYNCB_FIRST_KERNEL`; and `ASYNC_INITIALIZED`,
`ASYNC_NORMAL_ACTIVE`, `ASYNC_BOOT_AUTOCONF`, `ASYNC_CLOSING`,
`ASYNC_CTS_FLOW`, `ASYNC_CHECK_CD`, `ASYNC_SHARE_IRQ`, `ASYNC_CONS_FLOW`,
`ASYNC_INTERNAL_FLAGS`.

Local evidence: pinned `tty_flags.h:42-53` wraps the ten `ASYNCB_*` internal
bit-position macros in `#ifndef __KERNEL__`; pinned `tty_flags.h:84-95` wraps
the nine internal mask macros in the same condition. Therefore those
definitions are present to UAPI consumers but absent from a Linux kernel
translation unit. The candidate declares all nineteen as unconditional
`pub const`s at `tty_flags.rs:27-36` and `tty_flags.rs:66-74`, changing their
availability to kernel-side Rust callers.

The candidate additionally makes the unconditional Linux macro
`ASYNC_SUSPENDED` (`tty_flags.h:57`) evaluable in a kernel-side context by
unconditionally introducing its otherwise userspace-only operand
`ASYNCB_SUSPENDED`. In the pinned C header, kernel-side expansion of that
macro retains the original missing-operand behavior; the Rust constant instead
has a defined `u32` value. This is a source-level semantic/API difference.

Required resolution: represent the two `#ifndef __KERNEL__` groups with an
equivalent Rust visibility/configuration boundary, including the resulting
kernel-side status of `ASYNC_SUSPENDED`; do not expose a usable kernel-side
replacement for the guarded Linux definitions without pinned-source evidence
that the task's Rust module is exclusively a userspace-UAPI surface.

## Checked without additional finding

The eighteen unconditional user bit positions, eighteen direct mask macros,
six composite masks, SPDX/provenance fields, and named expressions in the
candidate match the pinned header's values and `1U`/unsigned-32-bit mask
semantics by manual source inspection. No symbols, types, linkage, layout,
allocation, locking, refcount, RCU, error, ordering, or branding behavior is
otherwise present in this macro-only header.

## Verdict

Rejected pending resolution of P1. This report completes parity-review slot 1
only; it makes no build or test claim.
