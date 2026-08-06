# Resolution — S016342

Applier reopened the complete pinned
`vendor/linux/include/uapi/linux/psample.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the header-closure entries for
both targets, the frozen `net/sched/.cls_api.o.cmd` commands, and the direct
`net/psample/psample.c` aggregate-initializer and typed-parameter contexts.
This was source-only work; no compiler, formatter, test, or runtime command
was run.

## Parity P1 — tagged enum surface

**Accepted and fixed.** Upstream lines 35–40 and 42–61 declare the distinct
public tags `enum psample_command` and `enum psample_tunnel_key_attr`; aliases
to `i32` erased that distinction. The final source uses separate public
`#[repr(transparent)]` wrappers with public `i32` fields. The wrappers retain
the frozen 4-byte C-`int` pass-by-value representation while avoiding Rust
fieldless-enum validity restrictions for values represented by C enum objects.
The frozen aarch64 and x86_64 `cls_api` commands contain no `-fshort-enums`.

The C enumerator names are unscoped `int` constant expressions, not scoped C
enum members. Accordingly all four command constants and all tunnel-key
constants remain public `i32` constants with their exact source-order numeric
expressions. This preserves `PSAMPLE_CMD_GET_GROUP + 1` as a numeric expression
and keeps the tags available for typed C ABI positions.

## Parity P1 / Rust R1 — string-literal macro expressions

**Accepted and fixed.** Upstream lines 66–68 expand to `char` array literals,
including a terminal NUL. The final source maps them to by-value `[u8; N]`
constants: `config\\0` has 7 elements; `packets\\0` and `psample\\0` have 8.
The frozen commands pass `-funsigned-char`, making `u8` the matching element
representation. This lets a translated aggregate initializer consume the array
value directly, as upstream does for `.name` in `net/psample/psample.c:33-34`
and `:111`; a translated pointer context obtains the character pointer at the
use site with `.as_ptr()`. No borrowed slice, reference expression, or named
static replaces these C literal macro values.

## Verified remaining source mapping

- The anonymous attribute enumeration is retained as `i32` constants 0 through
  17; `PSAMPLE_ATTR_MAX` remains the direct expression
  `__PSAMPLE_ATTR_MAX - 1`, evaluating to 16.
- `PSAMPLE_GENL_VERSION` remains the `i32` expression 1. No extra tunnel
  maximum macro exists upstream or was introduced.
- Both architecture header-closure rows select this header via
  `net/sched/cls_api.o`; it has no configuration conditional other than the
  include guard. All 22 S016342 symbol records are now `COMPLETE`.
- All six S016342 ABI and all six lifetime records are now `COMPLETE` with the
  named tag layout/alignment, by-value ownership, no destruction, and no
  synchronization explicitly recorded.

## Empty record families

`rewrite/DRIVER_ABI.tsv` has no S016342 row: this is a UAPI header, not a
`LINUX_DRIVER_OBJECT`, so the family is **N/A**. `rewrite/BLOCKERS.tsv` has no
S016342 row: the pinned source and frozen target command evidence resolve every
task question, so the family is **N/A**. There are no task-local statics,
functions, lock/RCU/refcount items, exports, or runtime cleanup records beyond
the enumerated symbol/ABI/lifetime rows.

All review findings are resolved; no finding remains open.
