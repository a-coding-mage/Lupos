# Applier resolution — S016427

## Inputs reopened

- Pinned source: `vendor/linux/include/uapi/linux/tty.h`, complete lines 1–46,
  at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen scope and queue: S016427 is the common `RUST_TRANSLATE` mapping to
  `src/include/uapi/linux/tty.rs`, selected by both frozen configurations
  through `rewrite/metadata/header_closure.tsv`.
- Candidate and both independent reports:
  `implementation.md`, `candidate.diff`, `parity-review.md`, and
  `rust-review.md`.

## Findings and dispositions

1. Parity review: **ACCEPT**, no findings. Independently confirmed: the pinned
   header has exactly the 31 object-like `N_*` integer-literal macros (0–30)
   plus `NR_LDISCS = 31`; the candidate exports the same 32 names and values.
   The original literal type is C `int` on both frozen targets, so every
   candidate item remains explicitly `core::ffi::c_int`. No conversion,
   expression evaluation, side effect, alias, or value drift was introduced.
2. Rust review: **ACCEPT**, no findings. The source defines only immutable
   object-like macro values. It has no function, object storage, structure,
   layout, linkage, unsafe operation, pointer, ownership, lifetime, locking,
   refcount, cleanup, error, or configuration-dependent behavior. Rust
   `pub const` items preserve the required compile-time value semantics; the
   C include guard has no separate Rust runtime or ABI item.
3. Consumer cross-check: the pinned `drivers/tty/tty_ldisc.c` uses
   `NR_LDISCS` as a range/array bound and uses `N_TTY` in signed
   line-discipline comparisons. The candidate's signed `c_int` constants
   retain those values and type category. Any future narrow-field conversion
   remains a caller concern, exactly as in C.

## Semantic-record closure

All 70 S016427 `SYMBOLS.tsv` rows (35 per frozen architecture) are resolved
against the pinned source lines and the frozen configuration/header-closure
evidence. The two conditional rows describe only the include guard; the guard
macro is a preprocessor inclusion mechanism with no Rust item or runtime ABI.
Each remaining operative macro is a pure signed C-`int` literal mapped to its
same-named `c_int` constant.

`LIFETIMES.tsv`, `ABI.tsv`, and `DRIVER_ABI.tsv` contain no S016427 rows:
this header has no lifetime-bearing, layout-bearing, external-linkage, or
driver-object contract to record. This is an N/A no-row family, not an omitted
pending record.

No source change was required during application. No build, compiler,
formatter, linker, test, runtime, debugger, emulator, or benchmark command
ran.
