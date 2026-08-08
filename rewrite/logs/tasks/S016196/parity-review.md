# Parity review — S016196 / P02 slot 1

Scope: `include/uapi/linux/ioam6_genl.h` against the current candidate
`src/include/uapi/linux/ioam6_genl.rs` and its supplied `candidate.diff`.
This was a manual source review only; no compiler, formatter, test, or runtime
tool was invoked.

## Result: findings require applier resolution

1. **High — C-string representation is not preserved for `IOAM6_GENL_NAME` and
   `IOAM6_GENL_EV_GRP_NAME`.**  Linux defines these symbols as C string-literal
   macros at `vendor/linux/include/uapi/linux/ioam6_genl.h:12,52`; each expands
   to a NUL-terminated character array/pointer expression.  The direct local
   consumer initializes `struct genl_family.name` with `IOAM6_GENL_NAME` and
   `struct genl_multicast_group.name` with `IOAM6_GENL_EV_GRP_NAME` at
   `vendor/linux/net/ipv6/ioam6.c:614,674`.  The candidate instead publishes
   Rust `&str` values.  A `&str` is a pointer-and-length value and does not
   promise an accessible trailing NUL, so it is not the same C-string
   representation at a Linux-compatible ABI boundary.  The applier must make
   the translated interface retain the literal bytes including the NUL and use
   a C-compatible pointer/array at every C-facing use, rather than relying on
   `&str`.

2. **High — ABI/lifetime record remains unresolved for `enum
   ioam6_event_type` and `enum ioam6_event_attr`.**  Linux declares ordinary C
   enums at `ioam6_genl.h:54-57,59-68`; `ioam6_event_type` is passed as the
   `type` argument of the external `ioam6_event` interface and forwarded to
   `genlmsg_put` (`vendor/linux/net/ipv6/ioam6.c:641-652`).  The candidate
   substitutes `#[repr(C)]` Rust enums, but the frozen ABI and lifetime records
   for both named enums on x86_64 and aarch64 are explicitly `PENDING_REVIEW`
   (`rewrite/ABI.tsv:192287-192292`, `rewrite/LIFETIMES.tsv:188228-188233`).
   No local source evidence in the candidate closes the required width, signed
   representation, or arbitrary-integer/invalid-discriminant behavior across
   the two ABIs.  Treating these as Rust enums can impose Rust discriminant
   validity where the Linux C interface transports an enum-compatible integer.
   This must be resolved from the pinned ABI context (or the task blocked), not
   assumed from `repr(C)`.

3. **High — the current candidate and the supplied candidate snapshot disagree
   on provenance for all header symbols, including `IOAM6_GENL_NAME`.**  The
   supplied `candidate.diff` records `//! architectures: x86_64,aarch64`, while
   the current destination records `//! architectures: common`.  The frozen
   task row records the task as `common` but the source-review provenance
   template requires the selected architecture identities; more importantly,
   the diff no longer describes the file being reviewed.  This discrepancy
   prevents an evidence-backed approval of the current definitions.  Regenerate
   the candidate snapshot from the intended source and re-review the final
   provenance before application.

## Checked without a discrepancy

The numeric sequence and maxima for `IOAM6_ATTR_*`, `IOAM6_CMD_*`, and
`IOAM6_EVENT_ATTR_*` match their Linux declaration order and values.  The
literal text of both protocol names, `IOAM6_GENL_VERSION`, and
`IOAM6_MAX_SCHEMA_DATA_LEN` also match.  No branding delta was observed.
