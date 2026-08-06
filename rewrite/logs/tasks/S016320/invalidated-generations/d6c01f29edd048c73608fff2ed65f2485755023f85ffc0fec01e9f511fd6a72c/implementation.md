# S016320 implementation record

- Task and pipeline: `S016320`, `P01`; lease status was `IN_PROGRESS` for
  `codex-root-cont5-20260806-p01` before implementation.
- Oracle: `vendor/linux/include/uapi/linux/oom.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`; Phase 0 identity records the
  same commit and common x86_64/AArch64 selection.
- Scope/map: `RUST_TRANSLATE`, common, no dependencies;
  `include/uapi/linux/oom.h -> src/include/uapi/linux/oom.rs`.
- Translation: the five selected C integer-expression macros are exported as
  `core::ffi::c_int` constants. Each source literal and unary-minus expression
  has C `int` type on both frozen targets, which `c_int` represents. The C
  include guard has no Rust runtime or ABI counterpart; Rust module inclusion
  provides the corresponding single-definition property.
- Context checked: the internal wrapper `include/linux/oom.h` and all direct
  in-tree uses of the five macros. They consume these values as signed integer
  score/adjustment bounds; no configuration conditional alters the header.
- No compiler, formatter, linker, test, runtime, or historical Lupos Rust
  source was used.
