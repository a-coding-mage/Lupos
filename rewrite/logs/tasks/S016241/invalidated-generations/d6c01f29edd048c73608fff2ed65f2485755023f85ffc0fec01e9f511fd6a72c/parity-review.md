# Parity review — S016241 (slot 1)

Reviewed `vendor/linux/include/uapi/linux/membarrier.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/membarrier.rs` for the frozen common x86_64/AArch64
scope.

## Result

No parity findings.

## Evidence checked

- The complete named `enum membarrier_cmd` declaration is represented by the
  C-`int` ABI alias `membarrier_cmd`.  All twelve enumerators are present with
  their source names and values: `QUERY` is zero; the ten command bits remain
  signed C-`int` shift expressions for bits 0 through 9; and the
  backward-compatible `SHARED` alias remains equal to `GLOBAL` (bit 0).
- The complete named `enum membarrier_cmd_flag` is represented by the C-`int`
  ABI alias `membarrier_cmd_flag`; its sole `MEMBARRIER_CMD_FLAG_CPU`
  enumerator remains the bit-0 value.
- Every source initializer is an `int` constant expression on both frozen
  targets. `core::ffi::c_int` is the corresponding C `int` ABI scalar, so the
  candidate retains the signed integer category and all evaluated values.
- The pinned header has only its conventional `_UAPI_LINUX_MEMBARRIER_H`
  include guard; it has no feature, Kconfig, architecture, or other selected
  conditional declaration. The guard has no separate Rust-module item.
- The source has no functions, structs, unions, storage definitions, linkage
  declarations, or executable behavior. The candidate adds no extra public
  UAPI declarations, unsafe code, test configuration, or branding delta.
- Immutable provenance names the exact source, pinned revision, `common`
  architecture scope, and task ID. The upstream copyright and permission
  notice are retained; the source contains no upstream SPDX identifier, while
  the required Rust provenance SPDX line is present.

No source, manifest, or non-review evidence file was modified by this
reviewer. No build, compiler, formatter, test, linker, debugger, or runtime
command was run.
