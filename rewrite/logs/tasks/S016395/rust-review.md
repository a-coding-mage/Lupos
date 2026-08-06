# Rust review — S016395, attempt 2

Reviewed `src/include/uapi/linux/sunrpc_netlink.rs` independently against
`vendor/linux/include/uapi/linux/sunrpc_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus the task-specific ABI,
lifetime, and symbol records for both frozen architectures.

## Result

PASS — no Rust-specific finding.

## Checks

- `sunrpc_cache_type` is represented by `core::ffi::c_int`; the frozen ABI
  records establish a four-byte signed C `int` representation on x86_64 and
  AArch64, with `-fshort-enums` absent.  The named and anonymous C enum
  declarations introduce no storage or lifetime-bearing object.
- Each enumerator is a `c_int` constant expression with the C source value,
  including every compiler-generated successor and each `__*_MAX - 1`
  expression.  The translation does not introduce a Rust `repr` enum whose
  validity rules would incorrectly exclude otherwise representable C integer
  values.
- `SUNRPC_FAMILY_NAME`, `SUNRPC_MCGRP_NONE`, and `SUNRPC_MCGRP_EXPORTD` retain
  their C string-literal array category: their exact bytes, terminating NUL,
  and lengths are represented as immutable `u8` arrays.  Pointer decay remains
  a consumer-site operation; no non-upstream pointer helper item remains.
- The header has no functions, structs, FFI linkage, mutable storage,
  synchronization, ownership transfer, callback, or `unsafe` boundary.  The
  static byte arrays have `'static` storage suitable for the C literal use
  sites and do not create a Rust aliasing or drop-time hazard.
- The Rust source has the required immutable provenance, no tests, no
  placeholder, and no additional public convenience surface.

No compiler, formatter, build, test, or runtime command was run.
