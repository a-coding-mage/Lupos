# Rust review — S014640

Reviewed only the pinned source and the candidate:

- Task: `S014640`, P01, `REVIEWING`; queue path `src/include/linux/pid_types.rs`, Linux path `include/linux/pid_types.h`.
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df` (`vendor/linux.SHA`).
- Frozen scope: `common`, selected for both x86_64 and aarch64.  `SYMBOLS.tsv` records the enum and `init_pid_ns` for both configurations.
- No compiler, formatter, rust-analyzer, linker, test, debugger, or runtime command was used.  No source or queue file was changed.

## Finding R1 — high: SPDX provenance does not retain the upstream identifier

`vendor/linux/include/linux/pid_types.h:1` declares `/* SPDX-License-Identifier: GPL-2.0 */`, while `src/include/linux/pid_types.rs:1` changes it to `// SPDX-License-Identifier: GPL-2.0-only`.

The rewrite protocol requires upstream SPDX identifiers to be retained.  This is an unauthorized source-provenance/license-identifier change even if the two spellings may be interpreted similarly by some tooling.  Restore the exact upstream identifier in the candidate.

## Finding R2 — medium: architecture provenance omits the two selected targets

The frozen task row identifies this common header as selected by both frozen configurations; `rewrite/SYMBOLS.tsv` has separate selected records for `aarch64` and `x86_64`.  The required provenance header records the architecture set, but `src/include/linux/pid_types.rs:4` says only `architectures: common` rather than `x86_64,aarch64`.

Correct the immutable provenance to identify both approved architectures explicitly.  This does not alter the header's source semantics, but it is necessary for auditable task provenance.

## Source/FFI checks with no additional finding

- `vendor/linux/include/linux/pid_types.h:5-11` defines five consecutive C-enum discriminants.  `src/include/linux/pid_types.rs:12-20` uses `#[repr(C)]` and exactly preserves the ordered values 0 through 4, including `PIDTYPE_MAX`; this is the appropriate C-layout representation on the two frozen target families.
- The C header forward-declares `struct pid_namespace` and declares the non-const external object at `vendor/linux/include/linux/pid_types.h:13-14`.  The private zero-length `#[repr(C)]` wrapper in `src/include/linux/pid_types.rs:29-32` does not expose fields or permit an external caller to construct it, and is suitable as an opaque pointee declaration for this header-only context.
- `unsafe extern "C" { pub static mut init_pid_ns: pid_namespace; }` at `src/include/linux/pid_types.rs:34-37` preserves the C symbol spelling, external linkage, object (not pointer) form, and mutability.  The `static mut` declaration requires unsafe access in Rust, avoiding an unsound safe mutable-global API.  The local kernel definition at `vendor/linux/kernel/pid.c:72-84` confirms it is a mutable `struct pid_namespace` object exported as `init_pid_ns`.
- The C include guard (`vendor/linux/include/linux/pid_types.h:2-3,16`) has no required Rust runtime or ABI counterpart.

## Verdict

Reject pending resolution of R1 and R2.  No source fixes were applied by this reviewer.

Reviewer: `rust_reviewer`  
Model: `gpt-5.6-terra`  
Reasoning effort: `high`  
Required queue completion action (coordinator only; not performed here): `python3 tools/rewrite_queue.py mark-review --id S014640 --slot 2 --pipeline P01`
