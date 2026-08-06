# Rust review — S013727

Reviewed source-only: `src/include/linux/device-id/platform.rs` against pinned
`vendor/linux/include/linux/device-id/platform.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for the frozen `common`
x86_64/AArch64 task.  Queue verification found this task in `REVIEWING` on
`P01`; no compiler, formatter, analyzer, build, or test was run.

## Finding R1 — `PLATFORM_MODULE_PREFIX` loses C token-concatenation semantics (medium)

`src/include/linux/device-id/platform.rs:27-31` models the object-like C macro
as a zero-argument Rust macro expanding to `b"platform:\\0"`.  A Rust invocation
must be written as `PLATFORM_MODULE_PREFIX!()` and evaluates to a byte-string
reference.  It cannot be adjacent to another string literal or passed as the
same preprocessing token sequence as the C object-like macro.

The pinned macro is `#define PLATFORM_MODULE_PREFIX "platform:"` at
`vendor/linux/include/linux/device-id/platform.h:10`.  Its token-level
expansion is observably required in the pinned tree: `scripts/mod/file2alias.c:962`
uses `PLATFORM_MODULE_PREFIX "%s"`, and
`drivers/gpu/drm/bridge/synopsys/dw-hdmi-cec.c:360` uses
`PLATFORM_MODULE_PREFIX "dw-hdmi-cec"`.  In C these become one concatenated
NUL-terminated string literal; neither call form can be represented by the
candidate macro.  The candidate's documentation at lines 23-26 also claims
the result is the original C string-literal expansion, which is not true for
these consumers.

The applier must preserve this selected macro's use-site semantics (including
the concatenated-literal consumers) rather than treat a byte-string expression
as a drop-in replacement.  This needs a source-level resolution coordinated
with the translation of each affected consumer; no compiler evidence is
requested.

## Checked without finding a defect

- `kernel_ulong_t` at candidate line 10 is `core::ffi::c_ulong`, matching the
  header's `unsigned long` at pinned line 6 on both frozen LP64 targets.
- `#[repr(C)]` and fields `[u8; 24]` then `c_ulong` at candidate lines 35-42
  preserve the header's field order, offset 24 for `driver_data`, 8-byte
  alignment, and total 32-byte layout.  The recorded x86_64 Kbuild command in
  `rewrite/metadata/x86_64/compile_commands.json` includes `-funsigned-char`,
  supporting the `u8` element representation; the same header is selected for
  both frozen architectures by `rewrite/SCOPE.tsv` row S013727.
- The type is a plain C data record; deriving `Copy, Clone` introduces no drop,
  ownership, aliasing, panic, or unsafe behavior absent from the C record.
- `PLATFORM_NAME_SIZE!()` returns `24i32`, matching the C `int` literal at
  pinned line 9, and its explicit conversion is limited to Rust's array-bound
  requirement.

Result: one medium-severity finding for applier disposition; no source edits
made by this reviewer.
