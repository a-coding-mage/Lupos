# S016024 applier resolution

Applier: `gpt-5.6-terra`, high reasoning effort.  This was a manual
source-only application review; no compiler, formatter, rust-analyzer, build,
test, debugger, or runtime tool was invoked.

## Pinned-source and frozen-scope check

- `vendor/linux.SHA` and the checked-out pinned Linux tree both identify
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The frozen scope row maps
  `include/uapi/asm-generic/sockios.h` one-to-one to
  `src/include/uapi/asm-generic/sockios.rs`, class `RUST_TRANSLATE`,
  architectures `common`, risk `low`.
- The frozen symbol inventory selects the C include guard and the seven
  value-only UAPI macros for both aarch64 and x86_64.  The include guard is a
  C multiple-inclusion mechanism, so it has no Rust UAPI item to emit.

## Review dispositions

| Review | Disposition | Upstream evidence |
| --- | --- | --- |
| Parity review | Accepted: no findings. | `include/uapi/asm-generic/sockios.h:6-12` defines exactly `FIOSETOWN`, `SIOCSPGRP`, `FIOGETOWN`, `SIOCGPGRP`, `SIOCATMARK`, `SIOCGSTAMP_OLD`, and `SIOCGSTAMPNS_OLD`, with consecutive values `0x8901` through `0x8907`. |
| Rust review | Accepted: no findings. | Each value fits C `int` on both frozen Linux targets; the candidate states each public value as `core::ffi::c_int`, with no layout, pointer, ownership, unsafe, or drop behavior introduced. |

## Independent final check

The candidate has one public Rust constant for each of the seven selected
Linux macro spellings, in the same order and with the unchanged values:
`FIOSETOWN=0x8901`, `SIOCSPGRP=0x8902`, `FIOGETOWN=0x8903`,
`SIOCGPGRP=0x8904`, `SIOCATMARK=0x8905`, `SIOCGSTAMP_OLD=0x8906`, and
`SIOCGSTAMPNS_OLD=0x8907`.  The timestamp comments continue to distinguish
`timeval` from `timespec` exactly as the source does.  No selected conditional
branch exists beyond the C guard.

The candidate preserves the source SPDX identifier
`GPL-2.0 WITH Linux-syscall-note` and has the required immutable provenance:
the precise Linux source path, frozen revision, `common` architecture set, and
task ID.  No branding allowlist entry applies.

All task-local `PENDING_REVIEW` items are now resolved: the seven macros map
one-to-one to `c_int` constants, the include guard is intentionally omitted as
a non-operative C preprocessing device, and this header supplies no ABI
layout, linkage, ownership, lifetime, locking, refcount, RCU, or semantic
dependency decision beyond those constants.  No source edit is required.

Result: accept the candidate unchanged and transition S016024 to `DONE`.
