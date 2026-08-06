# Parity review — S013736

Reviewed only the pinned source `vendor/linux/include/linux/device-id/spmi.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/device-id/spmi.rs`.  The frozen scope row selects this
header for both `aarch64` and `x86_64`; the recorded commands for both targets
define `__KERNEL__`, use `-funsigned-char`, and target LP64.

## Finding P1 — `SPMI_MODULE_PREFIX` does not retain the C macro's string-literal composition semantics (major)

Linux source line 10 defines the object-like macro as exactly
`"spmi:"`.  That replacement is a C string-literal token: adjacent C string
literals concatenate before the terminating NUL is formed, so an expression
such as `SPMI_MODULE_PREFIX "device"` denotes one `char[12]` string
`"spmi:device\\0"`.  It also retains C array-literal behavior in contexts
such as `sizeof`.

Candidate lines 27–31 instead define a function-like Rust macro whose only
expansion is `b"spmi:\\0"`.  A Rust byte string expression is a reference to
a fixed byte array, and the explicit NUL is already a data byte; it cannot
participate in C-style adjacent-literal token concatenation.  Consequently
the operative macro selected in `SYMBOLS.tsv` is not equivalent despite the
matching standalone byte sequence.  The accompanying claim at candidate
lines 23–26 that it expands to the original C string literal is therefore
incorrect.  Preserve the source macro's literal/composition contract (and
document any unavoidable Rust representation boundary) rather than exposing
a pre-terminated byte-slice expression as an equivalent macro.

## Checked parity points without findings

- The `__KERNEL__` conditional is selected by both frozen command families;
  candidate line 10 represents `unsigned long` with `c_ulong`, which is a
  64-bit unsigned LP64 word on both selected targets.
- `SPMI_NAME_SIZE` maps the source's `int` literal `32` and is used for the
  32-byte name array.
- With the recorded `-funsigned-char`, source lines 12–15 have bytes at
  offsets 0–31 and an 8-byte `kernel_ulong_t` at offset 32; candidate
  lines 39–45 use `#[repr(C)]`, `[u8; 32]`, and `c_ulong`, preserving the
  40-byte, 8-aligned layout on both frozen LP64 targets.
- The candidate carries the exact source path, frozen revision, architecture
  union, task ID, and retained SPDX identifier.

No compiler, formatter, analyzer, build, or test was run.
