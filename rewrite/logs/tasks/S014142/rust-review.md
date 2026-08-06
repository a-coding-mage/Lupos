# S014142 Rust semantics review

Reviewed `vendor/linux/include/linux/irqdomain_defs.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/irqdomain_defs.rs`, with the frozen x86_64 and AArch64
header context.

## Finding R1 — high: the signed enum ABI assertion is not established

The candidate makes `irq_domain_bus_token` a transparent wrapper over
`core::ffi::c_int` and states that the C enum has a frozen signed-`int` ABI.
The pinned header only supplies enumerators 0 through 15; it does not specify
the enum's compatible integer type. The frozen x86_64 and AArch64 commands
show no short-enum option, but that is insufficient to establish the signedness
of the C enum object type. The task rows in `rewrite/ABI.tsv` and
`rewrite/LIFETIMES.tsv` still record this type as `PENDING_REVIEW`.

This is ABI-significant because the tag occurs in object fields
(`include/linux/irqdomain.h:180,344` and `include/linux/msi.h:499`) and in
callbacks and declarations (`include/linux/irqdomain.h:101,103,371,384`). A
high-bit, non-enumerator object representation is negative through the Rust
`c_int` wrapper but may be interpreted as an unsigned C enum value. The
candidate deliberately permits such representations via its public tuple
field, so the signedness cannot be dismissed as unreachable.

The applier must obtain and record target-specific frozen ABI evidence for
size, alignment, and signedness, then retain or replace the representation to
match it. Do not close the task while the ABI record remains pending.

## Finding R2 — medium: SPDX identifier was changed

The upstream file begins `SPDX-License-Identifier: GPL-2.0`; candidate line 1
uses `GPL-2.0-only`. The rewrite rules require retaining SPDX identifiers, and
this task has no branding allowance for a license-identifier change. Restore
the upstream identifier.

## Source-level observations

Subject to R1, `#[repr(transparent)]` over one scalar has the scalar's layout
and alignment and adds no padding. Its public field preserves all `c_int` bit
patterns, including values outside the enumerator set, unlike a Rust value
enum. `Copy` is compatible with C scalar copying; `Clone`, `Eq`, and
`PartialEq` add no representation or drop behavior. The sixteen public
`c_int` constants have the correct successive values and preserve C's
unscoped integer-constant form.

Rust intentionally supplies no implicit `c_int`/wrapper conversion. Future
translations of C callers, fields, and switches must make those conversions
explicitly at the equivalent C conversion points; this header alone cannot
make the claim that all such uses already preserve C behavior.

No compiler, formatter, linker, test, emulator, debugger, or diagnostic tool
was run for this review.
