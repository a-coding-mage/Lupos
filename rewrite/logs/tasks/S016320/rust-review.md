# Rust semantic review — S016320

Reviewed candidate: `src/include/uapi/linux/oom.rs`  
Pinned source: `vendor/linux/include/uapi/linux/oom.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`  
Scope: common (`x86_64`, `aarch64`); pipeline `P01`; slot 2 (Rust review).

## Result

Accepted. No Rust-semantics, representation, visibility, configuration, or
provenance finding remains for this header.

## Evidence and disposition

| Pinned symbol / fact | Source evidence | Candidate disposition |
| --- | --- | --- |
| `OOM_SCORE_ADJ_MIN` | `oom.h:9`, macro expression `(-1000)` | Public `c_int` constant with the identical signed value. |
| `OOM_SCORE_ADJ_MAX` | `oom.h:10`, macro expression `1000` | Public `c_int` constant with the identical value. |
| `OOM_DISABLE` | `oom.h:16`, macro expression `(-17)` | Public `c_int` constant with the identical signed value. |
| `OOM_ADJUST_MIN` | `oom.h:18`, macro expression `(-16)` | Public `c_int` constant with the identical signed value. |
| `OOM_ADJUST_MAX` | `oom.h:19`, macro expression `15` | Public `c_int` constant with the identical value. |
| Header selection | `oom.h:2-3,21` is only the conventional include guard; no configuration conditional occurs | A Rust module needs no textual include guard. There is no omitted `cfg` condition for either frozen architecture. |
| Direct UAPI context | `include/linux/oom.h:9` includes this header; Phase 0 header closure selects it for both configurations, including `fs/coredump.o` | Candidate is a public module-level API. No C linkage, storage, or exported object is implied by source macros. |

The decimal literals in all five source macros fit the C `int` type. On both
frozen Linux targets (`x86_64-linux-gnu` and `aarch64-linux-gnu`), `c_int`
models that signed C ABI integer type; the candidate consequently preserves
the source expression values, signedness, and width. The source macros have
no parameters, aliases, composite expressions, side effects, casts, or
context-dependent evaluation to preserve beyond their `int` value. The
candidate intentionally gives Rust callers a monomorphic `c_int` constant;
that is the direct Rust representation of the C macro's already-`int` result,
not a replacement ABI symbol.

There are no structs, unions, bitfields, FFI declarations, `unsafe` blocks,
allocation, ownership, aliasing, synchronization, drop behavior, or error
paths in the source header. `ABI.tsv` and `LIFETIMES.tsv` contain no
task-specific rows, which is consistent with these pure preprocessor
constants. The source SPDX identifier, Linux source path, pinned revision,
architecture scope, and stable task ID are present and match the frozen task
record. The branding allowlist has no applicable entry and the candidate
preserves every Linux UAPI identifier.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or runtime
tool was invoked during this review.
