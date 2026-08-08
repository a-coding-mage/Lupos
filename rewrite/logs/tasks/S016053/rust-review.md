# Rust source review — S016053

Verdict: **FINDINGS — reject candidate pending applier correction.**

Reviewed manually against `vendor/linux/include/uapi/linux/arm_sdei.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen AArch64 records, and
the direct in-tree uses in `vendor/linux/arch/arm64/kernel/sdei.c` and
`vendor/linux/drivers/firmware/arm_sdei.c`. No compiler, formatter, test, or
runtime tool was invoked.

## RUST-S016053-01 — `SDEI_1_0_FN` loses defined unsigned-wrap behavior

Severity: high

Linux `SDEI_1_0_FN(n)` (source line 8) adds an `unsigned int`
`SDEI_1_0_FN_BASE` to its argument after C's usual arithmetic conversions. In
the natural `u32` domain, overflow is defined modulo `2^32`. The Rust macro at
candidate lines 12–16 expands to ordinary `+`. When inferred as `u32`, that
operation is build-configuration dependent: overflow is checked/panics where
Rust overflow checks are enabled rather than wrapping as the C expression
does. Its unconstrained `$n:expr` also does not reproduce C's usual arithmetic
conversions for signed or narrower operands.

The currently generated named constants do not overflow, but this is an
operative exported function-like UAPI macro, so equality on its listed literal
uses is insufficient. The applier must preserve the exact established argument
domain and defined wrap behavior without introducing a profile-dependent panic;
if the Rust interface deliberately narrows the C macro's domain, that boundary
requires explicit ABI/semantic evidence before closure.

Evidence: `vendor/linux/include/uapi/linux/arm_sdei.h:6-10`; candidate
`src/include/uapi/linux/arm_sdei.rs:8-18`.

## RUST-S016053-02 — Explicit Rust types alter the C UAPI macro expression surface

Severity: medium

The source header intentionally supplies untyped C macro tokens. On the
frozen AArch64 C model, small literals such as `SDEI_EVENT_REGISTER_RM_*`,
`SDEI_EVENT_STATUS_*`, `SDEI_EV_*`, GET_INFO selectors, event types, and
priorities are `int`; the major/minor masks are likewise `int`; and
`SDEI_VERSION_VENDOR_MASK` is `unsigned int`. The candidate instead exposes
these positive constants as `u32` (lines 71–90), exposes the major/minor masks
as `u64` (lines 37 and 39), and exposes the vendor mask as `u64` (line 41).

This is not just notation: Rust has no C usual arithmetic conversions, so the
explicit types determine comparisons, bit operations, casts, and FFI argument
compatibility for every Rust consumer. The direct firmware caller uses a `u64`
version word (`drivers/firmware/arm_sdei.c:960,975-978`), for which extractor
results happen to be numerically correct, but that caller does not establish a
license to change the UAPI macro types for other uses. The frozen ABI records
contain no completed type decision resolving this change. The applier must
establish and record the intended Rust representation for each macro family,
then make the source match that decision rather than silently applying one
unsigned width to all values.

Evidence: `vendor/linux/include/uapi/linux/arm_sdei.h:28-37,48-71`; candidate
`src/include/uapi/linux/arm_sdei.rs:36-41,71-90`; frozen `ABI.tsv` has no
S016053 ABI row closing these macro-type decisions.

## RUST-S016053-03 — Macro visibility is an unproved global-interface change

Severity: medium

Each function-like macro is marked `#[macro_export]` (candidate lines 11, 43,
50, and 57), placing it in the Rust crate root. The Linux definitions are
preprocessor macros made visible by including this particular UAPI header; they
do not create a separately exported linker or global module symbol. No frozen
ABI/symbol decision establishes that global Rust macro names and their collision
behavior are the faithful mapped interface. This is especially material for
`SDEI_1_0_FN`, whose behavior is itself found incorrect above.

The applier must either provide direct evidence for the chosen Rust macro scope
or retain header-scoped visibility through an equivalent mechanism. Do not
close this as style: macro name resolution is part of the callable interface.

Evidence: `vendor/linux/include/uapi/linux/arm_sdei.h:3-8,35-37`; candidate
`src/include/uapi/linux/arm_sdei.rs:11-16,43-62`.

## Other Rust-semantics checks

The candidate defines no structs, enums, unions, FFI functions, raw pointers,
references, `unsafe` blocks, allocation paths, callbacks, interior mutability,
or `Drop` implementations. Therefore layout, provenance, borrow duration,
pinning, Send/Sync, callback/interrupt lifetime, and unsafe-boundary checks are
not applicable to this file itself. The negative SDEI status values are
correctly represented as `i32`; this does not resolve the positive macro-type
finding above.
