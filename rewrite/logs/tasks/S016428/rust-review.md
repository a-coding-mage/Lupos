# Rust review: S016428

Reviewed `src/include/uapi/linux/tty_flags.rs` against pinned
`vendor/linux/include/uapi/linux/tty_flags.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding R1 — the Rust kernel surface exports C names excluded by `__KERNEL__` (high)

Upstream lines 42–53 put `ASYNCB_INITIALIZED`, `ASYNCB_SUSPENDED`,
`ASYNCB_NORMAL_ACTIVE`, `ASYNCB_BOOT_AUTOCONF`, `ASYNCB_CLOSING`,
`ASYNCB_CTS_FLOW`, `ASYNCB_CHECK_CD`, `ASYNCB_SHARE_IRQ`,
`ASYNCB_CONS_FLOW`, and `ASYNCB_FIRST_KERNEL` under `#ifndef __KERNEL__`.
Lines 84–95 similarly gate `ASYNC_INITIALIZED`, `ASYNC_NORMAL_ACTIVE`,
`ASYNC_BOOT_AUTOCONF`, `ASYNC_CLOSING`, `ASYNC_CTS_FLOW`,
`ASYNC_CHECK_CD`, `ASYNC_SHARE_IRQ`, `ASYNC_CONS_FLOW`, and
`ASYNC_INTERNAL_FLAGS`.  The frozen kernel configurations define
`__KERNEL__`, so these names are absent from the original kernel source
surface.

The candidate intentionally makes all 19 names unconditional `pub const`s
(lines 34–43 and 75–83).  Public constants become available to every Rust
kernel consumer and therefore cannot reproduce the original preprocessor
visibility contract.  Delete these declarations from the kernel translation
surface (or represent the precise build-mode separation without making the
names accessible in the frozen kernel configuration).  Do not treat their
historical UAPI presence as authorization to expose them to the Rust kernel.

## Checked items

- The 18 always-defined `ASYNCB_*` values are correctly typed as C `int`
  positions via `core::ffi::c_int`; `int` is 32 bits on both approved targets.
- The 18 direct flag expressions faithfully preserve C's `1U` operand width
  as `u32`.  All shifts are 0 through 16, so no Rust checked-shift, signed
  shift, or truncation change is introduced.
- `ASYNC_FLAGS`, `ASYNC_DEPRECATED`, `ASYNC_USR_MASK`, `ASYNC_SPD_CUST`,
  `ASYNC_SPD_WARP`, and `ASYNC_SPD_MASK` use the correct unsigned width and
  operand sets.  Forward reference to `ASYNC_SPD_MASK` is valid for Rust
  items and does not change its value.
- This header has no structs, FFI functions, storage, `unsafe`, ownership,
  aliasing, or layout concerns.

## Verdict

Reject pending R1.  No source was edited and no build, formatter, test, or
runtime command was run.
