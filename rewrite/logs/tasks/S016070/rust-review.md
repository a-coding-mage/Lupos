# Rust review — S016070

Reviewer: slot 2 (`rust_reviewer`)

Verdict: **REJECT**.  The numeric masks and values are accurate for the raw
primitive cases implemented, and this file has no `unsafe`, panic,
`todo!`/`unimplemented!`, or Rust test configuration.  However, the public
UAPI macro surface is not preserved.

## Findings

1. **[blocking] `BPF_CLASS`, `BPF_SIZE`, `BPF_MODE`, `BPF_OP`, and `BPF_SRC`
   have lost their C constant-expression contract.**  Upstream defines each as
   a function-like replacement expression, for example
   `#define BPF_SIZE(code) ((code) & 0x18)` in
   `vendor/linux/include/uapi/linux/bpf_common.h:17`.  When `code` is an
   integer constant expression, the result is also an integer constant
   expression.  That surface is consumed by the dependent UAPI construction
   macros in `vendor/linux/include/linux/filter.h:108-430`, which compose
   `BPF_OP`, `BPF_SRC`, and `BPF_SIZE` into instruction-field initializers.
   The candidate instead exposes ordinary generic `pub fn`s at
   `src/include/uapi/linux/bpf_common.rs:106,121,131,144,167`; their bodies
   dispatch through trait methods and therefore cannot be used in Rust const
   expressions, static initializers, pattern positions, or const-generic
   expressions.  This is not a semantics-preserving macro translation.  The
   applier must retain a const-capable expression facility while preserving
   the single evaluation and target-specific C conversion rules.

2. **[blocking] The sealed input trait narrows an unrestricted C integer
   expression to a fixed set of Rust primitive types.**  Each upstream macro
   is valid for any C integer expression; the C usual arithmetic conversions
   select the result type.  In particular, it accepts enumeration expressions
   and ABI/newtype representations used by dependent translations, in addition
   to the scalar types named in the candidate.  `BpfCodeMaskInput` is sealed by
   the private `Sealed` trait (`bpf_common.rs:7-10,20-100`), so a dependent
   UAPI module cannot add its corresponding representation.  The frozen
   target fact that `int` is 32-bit and the masks are positive does validate
   the candidate's `i32` results for `bool`, 8/16-bit inputs, and `i32`, and
   its same-width results for the listed wider primitives; it does not justify
   excluding the rest of the C macro's legal operand surface.  Restore a
   representation that maps every selected C operand category rather than
   introducing a closed, Rust-only API.

3. **[blocking] The `#ifndef BPF_MAXINSNS` branch is omitted.**  The source at
   `vendor/linux/include/uapi/linux/bpf_common.h:53-55` defines `4096` only as
   a default when the including translation unit has not already supplied
   `BPF_MAXINSNS`.  The candidate's unconditional
   `pub const BPF_MAXINSNS: i32 = 4096` at line 174 cannot represent that
   inclusion-time override.  `rewrite/SYMBOLS.tsv` records this exact
   conditional for both frozen architectures as `PENDING_REVIEW`; it must be
   resolved with an evidence-backed Rust configuration/interface mapping
   before this task can be `DONE`, not silently collapsed to the default.

## Checked facts

- Provenance names the pinned source and revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, matching `vendor/linux.SHA`.
- All listed object-like macro literals are representable as 32-bit signed
  `int` values on both frozen LP64 targets; their candidate values and masks
  match the source default definitions.
- No ownership, aliasing, FFI layout, `unsafe`, panic, or hidden test issue
  exists in the candidate itself.

No source, manifest, or queue file was edited by this reviewer.
