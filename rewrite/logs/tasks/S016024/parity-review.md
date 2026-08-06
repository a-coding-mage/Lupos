# Parity review — S016024 (slot 1)

Scope reviewed: `src/include/uapi/asm-generic/sockios.rs` against pinned
`vendor/linux/include/uapi/asm-generic/sockios.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Preconditions verified: required branch `feat/bun-like-rewrite-test`; queue row
`S016024` is `REVIEWING` for pipeline `P01`; source and destination paths match
the frozen scope row; architectures are `common`.

## Result: PASS — no parity findings

The Rust file contains exactly the seven operative UAPI names from the pinned
header, each with its exact value:

| Name | Linux value | Rust value |
| --- | --- | --- |
| `FIOSETOWN` | `0x8901` | `0x8901` |
| `SIOCSPGRP` | `0x8902` | `0x8902` |
| `FIOGETOWN` | `0x8903` | `0x8903` |
| `SIOCGPGRP` | `0x8904` | `0x8904` |
| `SIOCATMARK` | `0x8905` | `0x8905` |
| `SIOCGSTAMP_OLD` | `0x8906` | `0x8906` |
| `SIOCGSTAMPNS_OLD` | `0x8907` | `0x8907` |

Each source literal is an unsuffixed hexadecimal integer constant whose value
fits C `int` on both frozen targets.  The candidate exposes each as
`core::ffi::c_int`, preserving that expression type for the target C ABI.
The two timestamp comments retain their respective `timeval` and `timespec`
meaning.  No extra operative constants or selected branches are present.

The SPDX identifier exactly preserves `GPL-2.0 WITH Linux-syscall-note`.
The immutable provenance lines name the correct Linux source path, frozen
revision, `common` architecture membership, and task ID.  No branding delta is
involved or allowlisted.

This was a manual source-only review. No compiler, formatter, rust-analyzer,
build, test, debugger, or runtime command was used.
