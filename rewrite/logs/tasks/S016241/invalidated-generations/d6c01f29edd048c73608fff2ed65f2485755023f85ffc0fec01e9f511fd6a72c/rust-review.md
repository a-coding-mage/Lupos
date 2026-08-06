# Rust review — S016241

Reviewed `src/include/uapi/linux/membarrier.rs` against the complete pinned
`vendor/linux/include/uapi/linux/membarrier.h` as an independent Rust/ABI
review. No findings.

## ABI and declaration mapping

- The source's two declaration-only C enum tags are represented by distinct
  public aliases to `core::ffi::c_int`. This preserves the C `int` value ABI
  required by the UAPI command/flag interface without introducing a Rust
  discriminant layout, invalid-value restriction, or a different nominal
  representation.
- `membarrier_cmd` retains `QUERY = 0`, each command bit `1 << 0` through
  `1 << 9`, and `MEMBARRIER_CMD_SHARED = MEMBARRIER_CMD_GLOBAL`. The alias is
  therefore an equal `c_int` value, as in the C initializer.
- `membarrier_cmd_flag` retains `MEMBARRIER_CMD_FLAG_CPU = 1 << 0` with the
  same `c_int` representation.
- No casts, shifts, signedness-changing operations, packed/layout-bearing
  declarations, FFI functions, pointers, ownership, aliasing, unsafe blocks,
  allocation, panic path, or cleanup timing are present.

## Configuration and UAPI surface

- The only C preprocessor conditional is the include guard; Rust module
  loading supplies the corresponding one-definition behavior. The pinned
  x86_64 and AArch64 header contents contain no configuration-dependent
  declarations, so no `cfg` condition is required.
- All public enumerator names are retained exactly, including the documented
  backward-compatibility alias. No Linux-to-Lupos branding change appears.
- The required immutable provenance fields, SPDX identifier, and upstream
  copyright/permission notice are present. No Rust test configuration was
  added.

## Result

Accepted for Rust-semantics review: no source changes requested.
