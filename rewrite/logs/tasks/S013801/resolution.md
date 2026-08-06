# Resolution — S013801 (attempt 2)

Applier: `gpt-5.6-terra` (high)

## Source and frozen-task verification

- Branch: `feat/bun-like-rewrite-test`.
- `vendor/linux.SHA` and `vendor/linux` `HEAD` both resolve to
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The frozen S013801 row maps `include/linux/dqblk_v1.h` to
  `src/include/linux/dqblk_v1.rs`, class `RUST_TRANSLATE`, architectures
  `common`, with both frozen x86_64 and AArch64 header-closure evidence.
- Both independent reports were reread.  They request no source change; this
  source-only adjudication independently confirms that conclusion.

## Adjudication and final mapping

The complete upstream header consists of an include guard and exactly four
object-like macros.  The candidate has exactly the four corresponding public
constants, with the exact identifiers and values:

| Upstream replacement list | Rust mapping | Disposition |
| --- | --- | --- |
| `V1_INIT_ALLOC 1` | `pub const V1_INIT_ALLOC: core::ffi::c_int = 1` | accepted |
| `V1_INIT_REWRITE 1` | `pub const V1_INIT_REWRITE: core::ffi::c_int = 1` | accepted |
| `V1_DEL_ALLOC 0` | `pub const V1_DEL_ALLOC: core::ffi::c_int = 0` | accepted |
| `V1_DEL_REWRITE 2` | `pub const V1_DEL_REWRITE: core::ffi::c_int = 2` | accepted |

Each replacement list is a single unsuffixed decimal integer-literal token.
When expanded in a C expression, each literal has C `int` type and has no
operands, reads, writes, ordering constraints, repeated evaluation, or side
effects.  On the frozen x86_64 and AArch64 C ABI targets, the explicit
`core::ffi::c_int` mapping preserves that C-int scalar domain while each Rust
`const` remains a compile-time value rather than creating storage, C linkage,
or pointer provenance.  No precedence or macro-argument behavior exists to
preserve.  The only identified Linux consumers form the `DQUOT_*` `max(...)`
wrappers in `include/linux/quota.h`; the mapped values preserve their four
inputs exactly.

The `_LINUX_DQBLK_V1_H` definition and its `#ifndef`/terminal `#endif` are
preprocessor single-inclusion machinery only.  They create no runtime value,
ABI, linkage, layout, ownership, or configuration branch; Rust module
inclusion supplies the corresponding single-definition role.  No selected
configuration conditional occurs inside the guard.

## PENDING_REVIEW closures

For both `aarch64` and `x86_64`, the semantic records for `ifndef@6`,
`_LINUX_DQBLK_V1_H`, and `endif@15` are closed as `NOT_APPLICABLE` to runtime
or ABI semantics, with the include-guard treatment above.  The records for
`V1_INIT_ALLOC`, `V1_INIT_REWRITE`, `V1_DEL_ALLOC`, and `V1_DEL_REWRITE` are
closed as `COMPLETE` by the exact integer-literal mappings in the table.

There are no functions, types, statics, layouts, symbols, calls, allocation,
locking, RCU, refcount, lifetime, aliasing, unsafe, endian, error, cleanup, or
branding semantics in the pinned header.  Consequently no LIFETIMES or ABI
row applies and no blocker remains.

No source change was necessary.  No compiler, formatter, rust-analyzer,
linker, build, test, debugger, or runtime tool was used.
