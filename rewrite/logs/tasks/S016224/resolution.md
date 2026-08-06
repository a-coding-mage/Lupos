# Applier resolution — S016224

Independently rechecked the complete pinned
`vendor/linux/include/uapi/linux/limits.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its selected frozen
x86_64/AArch64 header-closure context, `src/include/uapi/linux/limits.rs`,
and both independent review reports. This is a source-only application.

## Review dispositions

| Report | Disposition |
| --- | --- |
| Parity review | Accepted. The source contains precisely thirteen public object-like UAPI limit macros; the candidate exports precisely those thirteen names and values, with no additional public limits item. |
| Rust review | Accepted. All replacement lists are unsuffixed decimal literals representable as signed C `int` on both frozen targets. `pub const ...: i32` preserves their value, signed width, and starting integer-expression type without creating storage, a link symbol, or an ABI item. |

## Independent source reconciliation

Pinned-source lines 5, 7--17, and 19 define exactly these unconditional,
object-like public UAPI macros, in source order:

| Name | C replacement list | Rust item |
| --- | ---: | --- |
| `NR_OPEN` | `1024` | `pub const NR_OPEN: i32 = 1024` |
| `NGROUPS_MAX` | `65536` | `pub const NGROUPS_MAX: i32 = 65_536` |
| `ARG_MAX` | `131072` | `pub const ARG_MAX: i32 = 131_072` |
| `LINK_MAX` | `127` | `pub const LINK_MAX: i32 = 127` |
| `MAX_CANON` | `255` | `pub const MAX_CANON: i32 = 255` |
| `MAX_INPUT` | `255` | `pub const MAX_INPUT: i32 = 255` |
| `NAME_MAX` | `255` | `pub const NAME_MAX: i32 = 255` |
| `PATH_MAX` | `4096` | `pub const PATH_MAX: i32 = 4096` |
| `PIPE_BUF` | `4096` | `pub const PIPE_BUF: i32 = 4096` |
| `XATTR_NAME_MAX` | `255` | `pub const XATTR_NAME_MAX: i32 = 255` |
| `XATTR_SIZE_MAX` | `65536` | `pub const XATTR_SIZE_MAX: i32 = 65_536` |
| `XATTR_LIST_MAX` | `65536` | `pub const XATTR_LIST_MAX: i32 = 65_536` |
| `RTSIG_MAX` | `32` | `pub const RTSIG_MAX: i32 = 32` |

Each source replacement list is a public C `int` constant expression, not a
typed storage object. The explicit `i32` declarations preserve that
source-level signed integer category. Contextual C conversions remain a
consumer-expression concern: for example, pinned UAPI consumers use
`NAME_MAX + 1` as an array bound in `include/uapi/linux/auto_fs.h` and use
`PATH_MAX` as the `path` array bound in
`include/uapi/linux/netfilter/xt_cgroup.h`. A translated consumer must make
the corresponding Rust array-length or target-type conversion locally; this
macro-only header must not substitute a `usize` or another consumer-specific
type for the original `int` expression.

`_UAPI_LINUX_LIMITS_H` and its opening/closing directives are solely C
multiple-textual-inclusion control. They introduce no public value, ABI, or
runtime behavior and correctly have no Rust item; the Rust module namespace
provides the once-defined module counterpart. No configuration conditional
encloses any of the thirteen macros, and both frozen configurations select the
same complete definition set. The source UAPI identifiers are retained
unchanged in the public Rust namespace. The candidate exactly retains the
source SPDX expression `GPL-2.0 WITH Linux-syscall-note` and required immutable
source, revision, common-architecture, and task provenance. The branding
allowlist is empty, and no branding difference was made.

## Final semantic-record closure

The 32 task-local `rewrite/SYMBOLS.tsv` records are closed by the preceding
source reconciliation: for each frozen architecture, the opening guard,
closing guard, and `_UAPI_LINUX_LIMITS_H` are C-preprocessor-only; each of the
thirteen named public macros is an unconditional signed C-`int` expression
with the mapped value above. These determinations apply identically to
x86_64 and AArch64. They close the Phase 0 `PENDING_REVIEW` semantic markers
as `COMPLETE` without changing the frozen Phase 0 manifest snapshot.

There are no S016224 rows in `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, or
`BLOCKERS.tsv`: the header declares no function, object, type, layout,
alignment, linkage, calling convention, allocation, ownership/lifetime,
cleanup, locking, RCU/refcount, callback, or driver contract. Every such
category is `NOT_APPLICABLE`.

No candidate source change was warranted. No compiler, formatter, build,
linker, test, emulator, debugger, benchmark, runtime command, or
compiler-backed analyzer diagnostic was run.
