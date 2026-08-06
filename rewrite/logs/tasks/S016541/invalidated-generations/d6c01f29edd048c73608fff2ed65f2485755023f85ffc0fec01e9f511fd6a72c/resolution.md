# Resolution — S016541

## Independent applier verification

I reopened the complete pinned source `vendor/linux/include/vdso/time64.h` at
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen task and
symbol records, both reviews, the candidate, frozen target triples, and the
selected architecture `bitsperlong` headers.  The candidate needs no source
change.

The complete operative source surface is eight object-like macros, mapped
one-for-one as follows:

| Linux macro | Pinned C spelling | Rust item | Disposition |
| --- | --- | --- | --- |
| `MSEC_PER_SEC` | `1000L` | `pub const MSEC_PER_SEC: core::ffi::c_long = 1_000` | accepted |
| `USEC_PER_MSEC` | `1000L` | `pub const USEC_PER_MSEC: core::ffi::c_long = 1_000` | accepted |
| `NSEC_PER_USEC` | `1000L` | `pub const NSEC_PER_USEC: core::ffi::c_long = 1_000` | accepted |
| `NSEC_PER_MSEC` | `1000000L` | `pub const NSEC_PER_MSEC: core::ffi::c_long = 1_000_000` | accepted |
| `USEC_PER_SEC` | `1000000L` | `pub const USEC_PER_SEC: core::ffi::c_long = 1_000_000` | accepted |
| `NSEC_PER_SEC` | `1000000000L` | `pub const NSEC_PER_SEC: core::ffi::c_long = 1_000_000_000` | accepted |
| `PSEC_PER_SEC` | `1000000000000LL` | `pub const PSEC_PER_SEC: core::ffi::c_longlong = 1_000_000_000_000` | accepted |
| `FSEC_PER_SEC` | `1000000000000000LL` | `pub const FSEC_PER_SEC: core::ffi::c_longlong = 1_000_000_000_000_000` | accepted |

`rewrite/PHASE0_IDENTITY.tsv` fixes the targets as `x86_64-linux-gnu` and
`aarch64-linux-gnu`.  Their selected Linux headers set `__BITS_PER_LONG` to
64, so every `L` literal is a signed 64-bit C `long`; each `LL` literal is a
signed 64-bit C `long long`.  The candidate preserves the category distinction
with `core::ffi::c_long` and `core::ffi::c_longlong`, and every value is exactly
representable in its corresponding signed type.

There are no operands, casts, function calls, storage accesses, or pointer
expressions in these macros.  Therefore their expansions have no evaluation
order, side-effect, aliasing, provenance, allocation, synchronization, or
cleanup behavior to preserve.  Rust `const` items likewise create neither
storage nor pointer identity; consumer arithmetic retains its own typed
conversion behavior at each use.  The include guard is a translation-unit
preprocessor mechanism only and has no Rust module, ABI, or runtime analogue.

The header declares no functions, data objects, layouts, linkage, or calling
conventions.  Consequently all S016541 `PENDING_REVIEW` symbol rows are closed
by this resolution: the guard conditionals and `__VDSO_TIME64_H` are
not-applicable in Rust, while all eight operative macro rows are resolved by
the mappings above for both frozen architectures.  No lifetime or ABI table
rows exist for this task.

## Review dispositions

1. Parity review: accepted.  Independent comparison confirms all eight names,
   literal values, and `L`/`LL` categories are preserved.
2. Rust review: accepted.  There are no ownership, unsafe, layout, FFI,
   aliasing, provenance, or evaluation-order findings.

No compiler, formatter, analyzer, build, linker, test, debugger, runtime, or
benchmark command was used during this application.
