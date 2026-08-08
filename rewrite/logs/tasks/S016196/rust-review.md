# Rust semantics review — S016196

**Verdict: REJECTED.** Manual source review only; no compiler, formatter, test, or diagnostic tool was invoked.

## Findings

### RUST-1 — C string-literal macros were changed into non-FFI Rust fat pointers (high)

`IOAM6_GENL_NAME` and `IOAM6_GENL_EV_GRP_NAME` in the candidate are `&str` values (candidate lines 13 and 33).  In the pinned UAPI header, they are C string-literal macros (upstream lines 12 and 52): each expands to a NUL-terminated character array when used, not to a Rust `(data pointer, length)` fat pointer and not to an unterminated UTF-8 slice.  The direct consumer initializes `genl_multicast_group.name` from `IOAM6_GENL_EV_GRP_NAME` at `vendor/linux/net/ipv6/ioam6.c:613-616`, and initializes `genl_family.name` from `IOAM6_GENL_NAME` at lines 673-675.  A Rust core/driver ABI consumer requiring the C representation cannot use these `&str` constants without a behavior-changing conversion and no trailing NUL is represented by the candidate.

Resolve by preserving the C string-literal representation required at each ABI use (including its trailing NUL), rather than publishing these protocol/ABI macros as `&str`; record the chosen representation and its consumer contract in the task ABI evidence.

### RUST-2 — Closed Rust `repr(C)` enums do not establish the C enum ABI or C value domain (high)

The candidate replaces the C tagged enums `enum ioam6_event_type` and `enum ioam6_event_attr` (upstream lines 54-68) with closed Rust enums (candidate lines 35-54).  The frozen ABI records for both named enums on x86_64 and AArch64 remain `PENDING_REVIEW` in `rewrite/ABI.tsv`; therefore there is no source-backed resolution that `#[repr(C)]` has the exact size, alignment, signedness, and calling ABI selected by the frozen C toolchain for these declarations.

More importantly, a C enum object may carry an integer outside the listed enumerators, whereas a Rust enum may only be inhabited by its declared discriminants.  This is material here: the direct consumer accepts `enum ioam6_event_type type` in `ioam6_event` and passes it to `genlmsg_put` before its `switch` (`vendor/linux/net/ipv6/ioam6.c:635-656`).  Reinterpreting an externally sourced or otherwise non-enumerator C value as the Rust enum would violate Rust's validity invariant, while the C implementation preserves the integer until its normal switch behavior.  `repr(C)` does not remove that Rust validity restriction.

Resolve the exact frozen C enum ABI and the permitted inbound/value-domain contract.  Unless source evidence proves values are always restricted to the listed set at every Rust boundary, represent the ABI-facing values with the resolved integer type plus named constants, not a closed Rust enum.  The task cannot be accepted while these ABI records remain pending.

## Checked areas

All numeric enumerator values and max-value arithmetic in the candidate match the pinned header text.  This file contains no unsafe blocks, ownership-bearing fields, allocation, callbacks, atomics, interior mutability, pinning, or `Drop` behavior to approve.  Those absences do not resolve RUST-1 or RUST-2.
