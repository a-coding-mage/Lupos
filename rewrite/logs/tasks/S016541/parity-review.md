# Parity review — S016541 (slot 1)

## Scope and evidence

- Reviewed destination: `src/include/vdso/time64.rs`.
- Compared only with pinned `vendor/linux/include/vdso/time64.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Verified the frozen queue row is `REVIEWING`, pipeline `P01`, with source
  `include/vdso/time64.h`, destination `src/include/vdso/time64.rs`, and
  architecture set `common`.
- Reviewed S016541's frozen symbol/ABI records for both `x86_64` and
  `aarch64`, plus the recorded header-closure context. No compiler, formatter,
  rust-analyzer, build, test, or runtime tool was used.

## Result

No parity findings.

The source header defines exactly eight object-like conversion macros and no
time structures, functions, storage, linkage, or callable ABI:

| Linux macro | C literal type | Candidate type | Value | Result |
| --- | --- | --- | --- | --- |
| `MSEC_PER_SEC` | `long` | `core::ffi::c_long` | 1000 | preserved |
| `USEC_PER_MSEC` | `long` | `core::ffi::c_long` | 1000 | preserved |
| `NSEC_PER_USEC` | `long` | `core::ffi::c_long` | 1000 | preserved |
| `NSEC_PER_MSEC` | `long` | `core::ffi::c_long` | 1000000 | preserved |
| `USEC_PER_SEC` | `long` | `core::ffi::c_long` | 1000000 | preserved |
| `NSEC_PER_SEC` | `long` | `core::ffi::c_long` | 1000000000 | preserved |
| `PSEC_PER_SEC` | `long long` | `core::ffi::c_longlong` | 1000000000000 | preserved |
| `FSEC_PER_SEC` | `long long` | `core::ffi::c_longlong` | 1000000000000000 | preserved |

`x86_64-linux-gnu` and `aarch64-linux-gnu` are both LP64 for the frozen
configuration union, so every `L` macro is a signed 64-bit C `long` and every
`LL` macro is a signed 64-bit C `long long`; the candidate's FFI types preserve
that distinction and width. The source literals have no operands, casts, side
effects, or evaluation-order behavior. The Rust constants likewise denote the
same fixed typed values; consumer-specific arithmetic conversion requirements
remain at each translated use, as they do for the typed C literals under C's
usual arithmetic conversions.

The source exposes no data layout, time structure, exported symbol, calling
convention, or other runtime time ABI in this header, and the candidate adds
none. Its SPDX, Linux source path, exact pinned revision, common architecture
membership, and task identifier match the task provenance requirements.

## Disposition

Accepted for parity review slot 1; no source change requested.
