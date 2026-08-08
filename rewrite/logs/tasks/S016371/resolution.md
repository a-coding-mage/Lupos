# S016371 application resolution — BLOCKED

Task: `include/uapi/linux/seg6_genl.h` ->
`src/include/uapi/linux/seg6_genl.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`  
Attempt/pipeline: `1` / `P02`  
Phase 0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`  
Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`

The candidate and the semantic-closure proposal are sealed.  They were not
changed.  The numeric anonymous-enum constants and their two derived maximum
values are source-consistent, but that does not establish the missing C-UAPI
boundary below.  The task cannot be closed without an exact bridge, and this
frozen task provides none.

## PARITY-001 — `SEG6_GENL_NAME` C string / `genl_family.name` binding

**Disposition: accepted; unresolved; BLOCKED.**

The pinned header defines `SEG6_GENL_NAME` as the C string literal `"SEG6"`
at `vendor/linux/include/uapi/linux/seg6_genl.h:5`.  Its sole operative pinned
consumer initializes `.name = SEG6_GENL_NAME` at
`vendor/linux/net/ipv6/seg6.c:494-497`; the destination member is
`char name[GENL_NAMSIZ]` at `vendor/linux/include/net/genetlink.h:78-82`, and
`GENL_NAMSIZ` is 16 at `vendor/linux/include/uapi/linux/genetlink.h:8`.

The sealed candidate instead declares `pub const SEG6_GENL_NAME: &str =
"SEG6";`.  This does not carry the literal's terminating NUL or provide the
fixed C character-array initialization required at the consumer boundary.
It is also not a C-layout character-array expression.  A source-proven repair
would require a frozen Rust representation of the generic-netlink family and
its exact initialization contract; none exists: the queue rows for
`src/include/net/genetlink.rs` (S015468),
`src/include/uapi/linux/genetlink.rs` (S016140), and
`src/net/ipv6/seg6.rs` (S017927) are all `TODO`, and all three destination
paths are absent.  Adding an array or an ad-hoc conversion in this header
would invent that unreviewed bridge and still not preserve the C macro
expression's use-site semantics.  No source change is therefore permissible
in this sealed attempt.

Affected sealed records remain unclosable:
`SC1-d9e89b57ec9ab851455ab7da16196e42004259912361c277d244bb1dfbf19b9d`,
`SC1-741a39a13b2e5845da0fd48f2ce5f97c0cab53e5e4a8bf18624cc75a164e19e1`,
`SC1-a6f8069867d1f4ae8994c4c51963a1c57f6ae926351acb52e8a36f94ad95a9db`,
and `SC1-b45752b16f7cc008f0c90cb7001ae4c88f7fdef7322911d9c81fcf0d45bd52b0`.

## PARITY-002 — include guard and C macro contract

**Disposition: accepted; unresolved; BLOCKED.**

The pinned header's `_UAPI_LINUX_SEG6_GENL_H` include guard is at lines 2-3
and 33.  It defines the C macros `SEG6_GENL_NAME` and `SEG6_GENL_VERSION` at
lines 5-6, `SEG6_ATTR_MAX` as `(__SEG6_ATTR_MAX - 1)` at line 20, and
`SEG6_CMD_MAX` as `(__SEG6_CMD_MAX - 1)` at line 31.  The direct consumer uses
these in distinct C token contexts: a policy array bound
(`seg6.c:140`), `genl_family` integer initializers (`seg6.c:497-498`), and
the `resv_start_op` expression (`seg6.c:504`).

The candidate's Rust constants preserve the evaluated integer values for Rust
callers, but neither reproduce C preprocessing, header-guard inclusion
behavior, nor macro expansion in the original consumers' C contexts.  The
frozen path mapping is solely to `src/include/uapi/linux/seg6_genl.rs`; there
is no frozen companion C-UAPI export or C/Rust macro bridge.  A new exported
header or bridge would expand the frozen design and require controlled scope
and ABI adjudication, not a silent correction.  The proposal's `COMPLETE`
dispositions for the guard and macros therefore cannot be accepted as closure.

## RUST-S016371-001 — Rust representation of the C literal

**Disposition: accepted; unresolved; BLOCKED.**

This is the same unrepresented boundary as PARITY-001, viewed from Rust's
layout and FFI semantics.  `&str` is not the fixed, NUL-terminated C character
sequence used by the pinned initializer.  No `#[repr(C)]` family type or
source-evidenced conversion site exists in the frozen Rust tree for this task;
the dependent generic-netlink and SEG6 task rows remain unstarted.  Providing
one locally would be a new unreviewed design, so the finding is not repaired
in this attempt.

## Verified but insufficient portions

The anonymous enums have no named enum type.  Their enumerators are C integer
constants with values `0..8` for attributes and `0..5` for commands; the
candidate's `i32` values, `SEG6_ATTR_MAX == 7`,
`SEG6_CMD_MAX == 4`, and `SEG6_GENL_VERSION == 1` match the pinned header.
The frozen ABI and lifetime records concern only the anonymous enum types and
can correctly be `NOT_APPLICABLE` for layout/lifetime.  They do not cover the
macro, header-guard, or C-string-family boundary established above.

No compiler, formatter, linker, test, runtime command, or Rust-analyzer
diagnostic was used.  The queue must remain `BLOCKED` until an authorized
scope/ABI bridge can establish exact C-UAPI macro and `genl_family.name`
semantics.
