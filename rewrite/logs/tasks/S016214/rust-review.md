# Rust review — S016214

Reviewer role: Rust reviewer (slot 2), source-only review.  No compiler,
formatter, rust-analyzer, linker, test, debugger, or runtime command was run.

Reviewed candidate: `src/include/uapi/linux/kdev_t.rs`.

Pinned source: `vendor/linux/include/uapi/linux/kdev_t.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Task-state verification: `S016214` is `REVIEWING`, maps
`include/uapi/linux/kdev_t.h` to `src/include/uapi/linux/kdev_t.rs`, is
`common`, and is owned by pipeline `P02`.

## Findings

### RUST-1 — major: the `__KERNEL__` selection condition was dropped

The source places all three UAPI macros inside `#ifndef __KERNEL__`
([`vendor/linux/include/uapi/linux/kdev_t.h:4-13`](../../../vendor/linux/include/uapi/linux/kdev_t.h)).
Consequently, an in-kernel inclusion contributes none of `MAJOR`, `MINOR`, or
`MKDEV`; `include/linux/kdev_t.h` supplies the distinct 20-bit kernel forms
instead.  The candidate unconditionally declares and re-exports all three
macros ([`src/include/uapi/linux/kdev_t.rs:15-34`](../../../src/include/uapi/linux/kdev_t.rs)).

This changes macro availability in the kernel translation and can create a
name collision or select the UAPI 8-bit encoding where the kernel source
would select no UAPI macro at all.  The frozen symbol inventory records this
condition as the selected `ifndef@4` branch for both architectures
(`rewrite/SYMBOLS.tsv`, `scope_id=S016214`, source line 4), so it needs an
explicit, faithful representation rather than omission.

### RUST-2 — major: direct Rust operators do not reproduce C integer promotions or operand compatibility

The candidate promises that its operands retain the C macro width and
signedness, but direct Rust operators do not apply C's integer promotions or
usual arithmetic conversions ([`src/include/uapi/linux/kdev_t.rs:11-13`,
`15-34`](../../../src/include/uapi/linux/kdev_t.rs)).  The pinned macros are
untyped C replacement lists ([`vendor/linux/include/uapi/linux/kdev_t.h:10-12`](../../../vendor/linux/include/uapi/linux/kdev_t.h)); before the C shift
or bitwise operators act, narrow integer operands are promoted to `int`, and
the two `MKDEV` operands are then subject to the usual arithmetic
conversions.

Examples of divergent valid C uses are `MAJOR((unsigned char)x)` and
`MKDEV((unsigned char)ma, (unsigned char)mi)`: C promotes the `unsigned char`
to `int`, so shifting by 8 is valid and produces an `int`; the Rust expansion
instead shifts a `u8` by its width.  Likewise, C can convert mixed-width
integer `MKDEV` operands to a common type, while `(($ma) << 8 | ($mi))`
requires Rust operands acceptable to the same built-in operation without that
conversion.  `MINOR` also changes the result type for narrow operands and can
reject signed narrow operands because `0xff` must be inferred for the Rust
bitwise operation.

The candidate macros additionally accept user-defined Rust types implementing
`Shr`, `Shl`, or `BitOr`, whereas C's replacement list invokes only C integer
operators.  This is a type-system and evaluation-contract expansion, not a
faithful UAPI macro translation.  The applier needs a representation that
preserves the selected C operand domain, promotions, result type, and
shift-bound behavior, or must block with a concrete ABI decision if that
cannot be expressed in the Rust-facing UAPI.

### RUST-3 — major: UAPI macro visibility is reduced to crate-private

The source explicitly says these are the definitions programs obtain from the
kernel sources and that they are externally visible
([`vendor/linux/include/uapi/linux/kdev_t.h:6-12`](../../../vendor/linux/include/uapi/linux/kdev_t.h)).
Each candidate macro is only re-exported as `pub(crate)`
([`src/include/uapi/linux/kdev_t.rs:20`, `27`, `34`](../../../src/include/uapi/linux/kdev_t.rs)),
which prevents any external Rust consumer from naming the macro through this
module.  It therefore cannot represent the source header's UAPI-facing
availability.  The required visibility/export mechanism must be resolved
alongside RUST-1 so it does not expose the macros in the kernel-only context.

## Manual checks with no finding

Each argument appears exactly once in the corresponding expansion, and the
parentheses preserve the C precedence of the three replacement lists.  The
file contains no `unsafe` code, raw pointers, references, allocation, or drop
logic; no provenance or ownership boundary is introduced by this candidate.

## Verdict

Reject pending resolution of RUST-1 through RUST-3.  These are source-level
semantic findings; no build or test claim is made.
