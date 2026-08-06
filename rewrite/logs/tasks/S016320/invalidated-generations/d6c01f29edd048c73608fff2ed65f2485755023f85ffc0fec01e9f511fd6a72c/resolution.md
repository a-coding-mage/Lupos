# Resolution — S016320

## Authority and recheck

I independently reopened the complete pinned source
`vendor/linux/include/uapi/linux/oom.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen task row and mapping,
both reviewer reports, the internal wrapper `include/linux/oom.h`, and direct
selected consumers in `fs/proc/base.c` and `mm/oom_kill.c`.  The task is
`RUST_TRANSLATE`, architecture scope `common`, with no dependencies; its
destination is `src/include/uapi/linux/oom.rs`.

No source change is required.  The candidate retains the source SPDX
identifier and required provenance, preserves every UAPI identifier, and
adds no UAPI object, linkage, or branding change.

## Review dispositions

| Report / finding | Disposition | Pinned-source basis |
| --- | --- | --- |
| Parity review: no findings | Accepted | `oom.h:9-10,16,18-19` contains exactly the five value macros represented by the candidate. |
| Rust review: no findings | Accepted | Each literal expression has C `int` type and fits it on both frozen targets; `core::ffi::c_int` is the matching signed C ABI type for the frozen x86_64 and AArch64 targets. |

The candidate maps the source expressions exactly: `(-1000)`, `1000`,
`(-17)`, `(-16)`, and `15` become public `c_int` constants named
`OOM_SCORE_ADJ_MIN`, `OOM_SCORE_ADJ_MAX`, `OOM_DISABLE`, `OOM_ADJUST_MIN`,
and `OOM_ADJUST_MAX`, respectively.  These macros are parameterless,
side-effect-free C `int` expressions, so no evaluation ordering, storage,
linkage, or macro-parameter behavior remains to reproduce.  Direct consumers
confirm their signed integer use: `fs/proc/base.c:1089-1101,1216-1229,1247-1276`
uses their signed bounds and legacy scaling; `mm/oom_kill.c:217-220,1000-1003,
1148-1151` compares the signed minimum.  No configuration conditional occurs
in the pinned UAPI header.

## Closed task-local semantic records

The following `rewrite/SYMBOLS.tsv` records for **both** `x86_64` and
`aarch64`, previously marked `PENDING_REVIEW`, are closed by this source-level
resolution:

| Record | Final disposition |
| --- | --- |
| `ifndef@2`, `endif@21`, `_UAPI__INCLUDE_LINUX_OOM_H` | They are the conventional C preprocessor include guard only. They create no runtime state, UAPI value, C object, ABI layout, or linkage; Rust module loading supplies the single-definition property. |
| `OOM_SCORE_ADJ_MIN`, `OOM_SCORE_ADJ_MAX` | Exact public signed `c_int` values `-1000` and `1000`. |
| `OOM_DISABLE`, `OOM_ADJUST_MIN`, `OOM_ADJUST_MAX` | Exact public signed `c_int` values `-17`, `-16`, and `15`. |

There are no task-local `ABI.tsv`, `LIFETIMES.tsv`, or `BLOCKERS.tsv` rows.
The header has no types, storage, FFI declarations, ownership, locks,
atomics, allocations, cleanup, or error path; therefore no additional ABI or
lifetime decision is pending.

This application used manual source inspection only. No compiler, formatter,
linker, analyzer diagnostic, test, runtime command, or historical Lupos Rust
source was used.
