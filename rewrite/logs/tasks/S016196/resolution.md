# S016196 adjudication — BLOCKED

Scope: `include/uapi/linux/ioam6_genl.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, current candidate
`src/include/uapi/linux/ioam6_genl.rs`, and the P02/a1 parity and Rust review
reports.  This adjudication reopened the complete pinned header, its local
wrapper, the selected `net/ipv6/ioam6.c` and `net/ipv6/exthdrs.c` consumers,
`include/net/ioam6.h`, and generic-netlink declarations.  It was source
inspection only; no compiler, formatter, linker, test, runtime, or historical
Lupos source was used.

## Evidence reopened

- `vendor/linux/include/uapi/linux/ioam6_genl.h:12,52` define
  `IOAM6_GENL_NAME` and `IOAM6_GENL_EV_GRP_NAME` as C string literals.  The
  selected consumer initializes `.name` from them at
  `net/ipv6/ioam6.c:614,674`.  The destination fields are fixed character
  arrays, not string slices: `struct genl_multicast_group.name` and
  `struct genl_family.name` are both `char name[GENL_NAMSIZ]` at
  `include/net/genetlink.h:29-32,78-81`.  Thus their source initializer
  semantics include the literal terminator and aggregate zero-fill.
- The complete header declares named C enum tags at lines 54--57 and 59--68.
  The selected declaration in `include/net/ioam6.h:71-72` and definition in
  `net/ipv6/ioam6.c:635-666` pass `enum ioam6_event_type` by value; the latter
  passes it onward to `genlmsg_put`, whose command parameter is `u8`
  (`include/net/genetlink.h:336-337`), then switches on it.  This establishes
  that the value is consumed as an integer protocol command, but does not
  establish the object representation, alignment, or C calling ABI of either
  named enum.
- Frozen `ABI.tsv` rows 192287--192288 (aarch64) and 192291--192292 (x86_64)
  retain `PENDING_REVIEW` for the layout and alignment of both named enum tags.
  The corresponding `LIFETIMES.tsv` rows 188228--188229 and 188232--188233
  also remain pending.  No local frozen ABI artifact supplies the missing
  target values.
- The evidence snapshot is stale: `candidate.diff` records
  `//! architectures: x86_64,aarch64`, while the current candidate records
  `//! architectures: common`.  The frozen task row uses `common`, while the
  immutable source provenance template requires the selected x86_64 and
  aarch64 identities.  The supplied diff therefore cannot be accepted as a
  snapshot of the current candidate.

## Finding dispositions

### P1 — C-string representation (parity review): accepted

`&str` is not a C string-literal/fixed-array initializer: it omits the
terminating byte and has a Rust fat-pointer representation.  A future candidate
must preserve each literal's bytes plus its terminating NUL under the original
public name, in a representation that can initialize the cited fixed
`char[GENL_NAMSIZ]` fields with the source's zero-fill behavior.

### RUST-1 — C string-literal FFI representation (Rust review): accepted

This independently identifies the same source defect as P1.  It has the same
required correction and no separate implementation disposition.

### P2 — named-enum ABI/lifetime closure (parity review): accepted; blocking

The current closed Rust `#[repr(C)]` enums neither prove the frozen C layout
nor preserve the C integer value domain.  The upstream event path shows a
by-value enum parameter used as a protocol command, but it cannot answer the
missing size, alignment, signed representation, or call ABI for either
architecture.  The required ABI and lifetime records remain pending.  Inferring
`i32`, accepting `repr(C)`, or substituting an integer without target evidence
would be an unreviewed ABI design.

### RUST-2 — closed Rust enum validity domain (Rust review): accepted; blocking

This independently identifies the same unresolved ABI/value-domain defect as
P2.  The C source's conversion to `u8` and ordinary `switch` behavior does not
make a Rust fieldless enum valid for all C integer states, and no frozen source
evidence constrains every Rust boundary to listed discriminants.

### P3 — candidate snapshot/provenance mismatch (parity review): accepted

The final candidate cannot be applied from a diff that describes a different
provenance line.  Once the ABI blocker is resolved and a corrected candidate is
written, regenerate `candidate.diff` from that exact file and obtain fresh,
independent reviews; the current reports do not approve an unseen replacement.

## Final disposition

The string and snapshot defects are controlled requeue work, but they cannot
close S016196 because the named-enum ABI evidence is absent for both frozen
targets.  Phase 1 may not create compiler evidence or guess an ABI.  The
source-backed disposition is therefore **BLOCKED**, not `DONE` and not a
source requeue at this time.

To resume, obtain auditable Phase 0 ABI evidence bound to the existing Phase 0
identity for `enum ioam6_event_type` and `enum ioam6_event_attr` on x86_64 and
aarch64, including representation, size, alignment, calling-boundary behavior,
and the permitted value-domain contract.  Then requeue for the two specified
source corrections, regenerate the exact candidate snapshot, and perform fresh
independent reviews.

No source, queue, or semantic-closure manifest was modified by this
adjudication.
