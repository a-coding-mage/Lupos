# S016196 parity review (slot 1)

Reviewed the complete pinned
`vendor/linux/include/uapi/linux/ioam6_genl.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/ioam6_genl.rs`.

## Result

PASS — no parity findings.

## Checked surface

- The candidate has the exact upstream UAPI SPDX expression and immutable
  provenance for the source path, pinned revision, common architecture set,
  and task `S016196`.  It introduces no branding change, configuration branch,
  function, structure, test, placeholder, or executable behavior.  The source
  include guard has no Rust-module runtime/API counterpart.
- All eight anonymous attribute enumerators are present as C-`int` constants:
  `IOAM6_ATTR_UNSPEC` through `IOAM6_ATTR_PAD` retain values 0 through 7;
  `__IOAM6_ATTR_MAX` is 8 and `IOAM6_ATTR_MAX` still expresses sentinel minus
  one.  `IOAM6_MAX_SCHEMA_DATA_LEN` remains the C `int` expression/value
  `255 * 4` (1020).
- All nine anonymous command enumerators are present as C-`int` constants:
  `IOAM6_CMD_UNSPEC` through `IOAM6_CMD_NS_SET_SCHEMA` retain values 0 through
  7; `__IOAM6_CMD_MAX` is 8 and `IOAM6_CMD_MAX` remains sentinel minus one.
  This agrees with the actual generic-netlink operation table in
  `vendor/linux/net/ipv6/ioam6.c` (including the `resv_start_op` expression).
- Both named UAPI enum tags are retained as distinct transparent `c_int`
  types, rather than being collapsed into untyped numbers.  Their enumerator
  constants have the exact tag and values: `IOAM6_EVENT_UNSPEC`/`TRACE` are
  0/1, and `IOAM6_EVENT_ATTR_UNSPEC`, `TRACE_NAMESPACE`, `TRACE_NODELEN`,
  `TRACE_TYPE`, `TRACE_DATA`, and `__IOAM6_EVENT_ATTR_MAX` are 0 through 5;
  `IOAM6_EVENT_ATTR_MAX` is still the sentinel minus one.  The tagged
  `ioam6_event_type` form also matches the upstream `ioam6_event()` parameter
  and switch use in `net/ipv6/ioam6.c`.
- `IOAM6_GENL_VERSION` remains `c_int` value 1.  The two C string-literal
  macros retain exact immutable, NUL-terminated `c_char` arrays: `"IOAM6"`
  has six bytes and `"ioam6_events"` has thirteen.  This preserves their
  storage and ordinary pointer-decay role at the upstream generic-netlink
  family and multicast-group name fields without replacing them with Rust
  `&str` values.

`rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` retain Phase-0
`PENDING_REVIEW` rows for this header's enum groups.  The candidate contains
the source-faithful decisions above; the applier must record the corresponding
final manifest resolutions before `DONE`, as required by the workflow.

No source, build, formatting, test, or runtime command was run during this
source-only review.
