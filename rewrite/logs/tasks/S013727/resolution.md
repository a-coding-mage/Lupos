# Resolution — S013727

**Disposition: BLOCKED.** This is source-only applier adjudication for
S013727/P01 attempt 2 on `feat/bun-like-rewrite-test`.  The oracle reopened in
full is `vendor/linux/include/linux/device-id/platform.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` (lines 1--17).  The queue row is
`APPLYING` for that same task, destination, and common x86_64/AArch64 scope.
No compiler, formatter, linker, test, emulator, debugger, rust-analyzer
diagnostic, historical Lupos source, or non-leased source edit was used.

## Review findings

| Finding | Disposition | Pinned evidence and resolution |
| --- | --- | --- |
| Parity P1 / Rust 1: `PLATFORM_MODULE_PREFIX` was represented as an already-NUL-terminated byte slice | Accepted; blocking | Oracle line 10 is the object-like C macro replacement list `"platform:"`, not a declared object or a pre-terminated prefix buffer.  C adjacent-literal composition forms exactly one final string-literal array and terminating NUL.  Frozen uses demonstrate the required contract: `scripts/mod/file2alias.c:962` passes `PLATFORM_MODULE_PREFIX "%s"`; `drivers/gpu/drm/bridge/synopsys/dw-hdmi-cec.c:360` passes `PLATFORM_MODULE_PREFIX "dw-hdmi-cec"` to `MODULE_ALIAS`; and `drivers/base/platform.c:1409` uses it in the `MODALIAS=%s%s` format.  A `&[u8; 10]` is neither a delimiter-free literal token nor a C array expression: it places `\0` before a suffix, cannot participate in adjacent-literal composition, has reference rather than array/decay behavior, and does not preserve `sizeof` behavior.  The invalid byte-slice substitute has therefore been removed rather than retained as a false mapping. |
| Rust 2: `platform_device_id` lacked ordinary C aggregate copy behavior | Fixed | Oracle lines 12--15 define an aggregate of a fixed `char[24]` and `kernel_ulong_t`, with no nontrivial copy or destruction operation.  `#[derive(Copy, Clone)]` restores independent ordinary by-value use of this Rust representation without changing `#[repr(C)]`, either field, ordering, alignment, or storage.  The direct core context preserves pointers to table entries (`drivers/base/platform.c:1145--1151`; `include/linux/platform_device.h:277`), but that pointer use does not prohibit ordinary value copies of the aggregate. |

## Semantic-record disposition

The frozen Phase 0 TSVs are evidence and outside this applier's authorized
write scope.  The following source-backed conclusions are recorded here for
both frozen architectures:

| Record(s) | Disposition |
| --- | --- |
| Include guard / `#ifndef` / matching `#endif` | C preprocessing-only include-once control; it has no runtime, storage, linkage, or Rust ABI counterpart. |
| `__KERNEL__` and `kernel_ulong_t` | The selected in-kernel branch makes the typedef C `unsigned long`.  Both frozen targets are LP64, so the represented word is `u64`, size/alignment 8, with no independent storage, ownership, lifetime, lock, RCU, refcount, or callback contract. |
| `PLATFORM_NAME_SIZE` | The C unsuffixed literal is signed `int` value 24; its Rust use as an array bound is an explicit representation-bound conversion only. |
| `struct platform_device_id` | `#[repr(C)]`, `[u8; 24]` at offset 0, then `u64` at offset 24, gives alignment 8 and total size 32 on both frozen 64-bit targets.  It has no packed, union, flexible-array, allocation, ownership-transfer, lock, RCU, refcount, callback, drop, section, or declared-symbol contract.  Storage duration and lifetime belong to the enclosing ID table/object; `driver_data` remains an opaque word whose later pointer/integer uses must establish provenance at their own conversion sites.  It is ordinarily copyable by value. |
| `PLATFORM_MODULE_PREFIX` | **Unclosed, blocking.** It has literal-token composition, exactly-one-final-NUL, C string-literal array, pointer-decay, and `sizeof` semantics at each expansion.  No audited Phase 0 mapping specifies a Rust source construct that preserves all of those semantics without rewriting the consumer interface. |

## Blocking condition

Rust declarative macros require invocation delimiters and cannot serve as a
delimiter-free object-like replacement token.  A suffix-taking macro,
`concat!` wrapper, byte slice, static, raw pointer, or helper API would require
rewritten call sites and changes one or more of literal composition, final-NUL,
array/`sizeof`, or pointer-decay behavior.  No frozen manifest, ABI record,
or porting rule audits such a cross-language mapping for this public operative
macro.  The task must remain blocked until scope/source guidance supplies one
that covers every translated consumer, or mechanically establishes that this
macro is not a required operative contract and reopens the frozen manifests.
Do not replace it with a byte slice or other convenience API.

The corrected record-copy behavior does not resolve the macro contract; hence
there is no `DONE` claim and no assertion of compile, link, test, boot, or
runtime parity.
