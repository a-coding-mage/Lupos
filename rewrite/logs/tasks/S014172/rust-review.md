# Rust review — S014172 / P02 / attempt 1

Reviewer role: `rust_reviewer`  
Model: `gpt-5.6-terra` / high  
Scope reviewed: pinned `vendor/linux/include/linux/kern_levels.h`, candidate
`src/include/linux/kern_levels.rs`, candidate snapshot, frozen task manifests,
and the pinned `printk` consumer/parsing context. No compiler, formatter,
analyzer, runtime, or test was invoked.

## Verdict: FINDINGS — exact Rust mapping is not established

### R1 — C string-array/NUL and pointer contract is absent (blocking)

`KERN_SOH`, `KERN_EMERG` through `KERN_DEBUG`, `KERN_DEFAULT`, and `KERN_CONT`
in the C header participate in adjacent C string-literal formation. The result
is a terminated `char[]` suitable for the `const char *` format/prefix paths.
Pinned `include/linux/printk.h:20-50` indexes `const char *buffer`, and pinned
`kernel/printk/printk.c:2178-2225` traverses terminated text while parsing the
two-byte prefix. The candidate instead expands to Rust `&'static str` values,
for which neither a trailing NUL byte nor a C-pointer representation is part of
the value contract. `concat!` produces a Rust string, not a Linux C string
array with its terminating byte. The candidate supplies no `#[repr(C)]`
byte-array/static or pinned conversion boundary that preserves the required
prefix bytes plus terminator without changing caller behavior.

This is not an ownership-only omission: manufacturing a borrowed C pointer from
the Rust `str` would also be invalid unless a terminated allocation/static and
its lifetime are explicitly established. No frozen ABI or lifetime record for
S014172 supplies that contract. Do not close the related semantic slots as
complete.

### R2 — macro invocation and adjacent-literal composition are not equivalent (blocking)

The C macros are object-like tokens. A pinned consumer may write
`printk(KERN_CONT "...", ...)`; preprocessing expands `KERN_CONT` and C then
joins the neighboring string literals into one format literal before the call.
The candidate changes every prefix to a function-like Rust macro that requires
`KERN_CONT!()` and emits a `&str`. Rust cannot place that invocation adjacent to
a string literal to recreate the C spelling/formation rule; it requires an
explicit alternative such as `concat!(KERN_CONT!(), "...")`. The candidate
does not provide a source-level mapping for all such consumers nor an
equivalent formatting/FFI macro layer. Consequently the candidate changes both
the token interface and the construction/evaluation interface of every
`KERN_*` prefix macro.

### R3 — header-global macro visibility is not established in the Rust module tree (blocking)

The C include guard controls repeated textual inclusion while making the
object-like `KERN_*` macros available throughout each including translation
unit. Plain `macro_rules!` declarations have Rust textual/module visibility;
they are not `pub` items and are not path-exported by this candidate. The
candidate's comment asserts a path-local module boundary represents the C
guard, but the frozen task evidence supplies no generated module index or
macro-import/visibility contract that lets all of the header's selected users
resolve these macros without namespace collisions or changed use syntax. This
is especially material because `include/linux/printk.h` itself composes these
macros into the parser/call macros above.

## Non-blocking observations

The numeric `LOGLEVEL_*` values and `KERN_SOH_ASCII`'s numeric value match the
pinned literals for x86_64 and AArch64. The issue is the missing C string and
preprocessor/consumer ABI, not the integer values. The candidate has no
unsafe blocks, ownership-bearing values, callbacks, or `Drop` behavior to
approve.

## Required disposition

Block S014172 unless the frozen scope/ABI/lifetime and consumer translation
records establish an exact common Rust representation and caller mapping for:

1. static NUL-terminated prefix byte arrays and valid C-pointer lifetime;
2. every selected `KERN_*` use that relied on C adjacent-literal preprocessing;
3. cross-module visibility/guard behavior for this header's macros.
