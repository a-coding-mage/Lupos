# Parity review — S016224

Reviewer: parity reviewer (P02, slot 1)  
Scope: `include/uapi/linux/limits.h` → `src/include/uapi/linux/limits.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`

## Result

No parity findings.

## Evidence checked

- The candidate provenance identifies the exact source path, pinned revision, `common` architecture scope, and task ID.  Its SPDX expression exactly preserves `GPL-2.0 WITH Linux-syscall-note`.
- The upstream header has one include guard and thirteen public object-like macros.  The candidate has exactly the thirteen public names: `NR_OPEN`, `NGROUPS_MAX`, `ARG_MAX`, `LINK_MAX`, `MAX_CANON`, `MAX_INPUT`, `NAME_MAX`, `PATH_MAX`, `PIPE_BUF`, `XATTR_NAME_MAX`, `XATTR_SIZE_MAX`, `XATTR_LIST_MAX`, and `RTSIG_MAX`; it adds no public limits definition.
- Every value is identical to the upstream unsuffixed decimal literal: 1024, 65536, 131072, 127, 255, 255, 255, 4096, 4096, 255, 65536, 65536, and 32 respectively.  Each upstream literal has C type `int` on the frozen x86_64 and AArch64 targets; each Rust constant is explicitly `i32`, preserving that 32-bit signed integer representation.  Context-specific C conversions are performed by the consuming expression, not by these object-like macro definitions; corresponding Rust consumers must make their target-width conversion explicitly.
- `_UAPI_LINUX_LIMITS_H` is solely the C preprocessing include sentinel.  The Rust module is loaded once by its module namespace, so omission of a Rust item with that sentinel name introduces neither an ABI nor a UAPI namespace difference.
- There are no configuration conditionals around the public macros.  `SYMBOLS.tsv` selects the same guard and all thirteen macros for both frozen configurations.  `ABI.tsv` and `LIFETIMES.tsv` contain no ABI or lifetime records for this constants-only header.
- Direct upstream inclusions are `include/linux/limits.h` and `kernel/auditsc.c`; the latter is selected for both frozen configurations (`CONFIG_AUDIT=y`, `CONFIG_AUDITSYSCALL=y`).  Neither adds a conditional definition or alters any of these values.  The internal wrapper adds kernel-only limits after including this UAPI header, which is outside this task and does not collide with the candidate namespace.

No compiler, formatter, analyzer, build, or test was run.
