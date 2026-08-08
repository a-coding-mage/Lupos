# Application resolution — S016142 (attempt 2, P02)

## Outcome

**BLOCKED — do not mark DONE or accept the semantic-closure proposal.**

The current candidate is not an exact translation, and its repairable defects
do not remove the unresolved named-enum ABI.  A controlled requeue would be
appropriate for the first two defects only if the enum ABI were independently
established; it is insufficient for this task as a whole.  No source, queue,
or frozen-manifest mutation was made in this application pass.

## Reopened evidence

- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
  (`vendor/linux.SHA`).
- Complete oracle: `vendor/linux/include/uapi/linux/handshake.h:1-76`.
- Current candidate: `src/include/uapi/linux/handshake.rs` (SHA-256
  `60055023ae51de534716e4b9640722f326d53dc9a3f5fb1ad7b7b9a462148c07`).
- Direct selected consumers recorded by
  `rewrite/metadata/header_closure.tsv`: `net/handshake/genl.c`,
  `netlink.c`, `request.c`, and `tlshd.c` on both architectures.
- Frozen `ABI.tsv` rows 191233-191235 (aarch64) and 191240-191242
  (x86_64), plus `LIFETIMES.tsv` rows 187174-187176 and 187181-187183,
  retain `PENDING_REVIEW` for the three named enum types.

## Finding dispositions

### P1 — C-string macro contract

**Accepted; controlled requeue repair required, but not sufficient for final
closure.**

`handshake.h:10,73-74` defines C string literals.  The current candidate's
`&str` values at `handshake.rs:7,64-65` omit the terminating NUL and are Rust
fat slices, not the macro's character-array/pointer-context value.  The source
also demonstrates a concrete fixed-array use: `net/handshake/genl.c:50-59`
initializes `struct genl_family.name`, declared as `char name[GENL_NAMSIZ]` in
`include/net/genetlink.h:78-81`, from `HANDSHAKE_FAMILY_NAME`.

A reimplementation must retain the exact NUL-terminated bytes for all three
macros and map each C use at its boundary; it must not expose `&str` as the
UAPI/FFI representation.  This is source-established, but it was not applied
because this pass is resolution-only.

### P2 — global named enumerator constants

**Accepted; controlled requeue repair required, but not sufficient for final
closure.**

The enumerators in `handshake.h:13-30` are introduced at file scope.  The
candidate instead makes their spellings enum variants only.  That loses the
unqualified integer-expression interface used by the pinned source:
`net/handshake/request.c:112-119` compares `handshake_proto.hp_handler_class`,
declared `int` in `net/handshake/handshake.h:54-57`, with the handler-class
enumerators; `net/handshake/tlshd.c:245-268,295-413` assigns and switches on
the other named enumerators through `int` fields.

A reimplementation must provide the named enumerator spellings as top-level
primitive integer constants with the source values 0..2 for handler class and
message type, and 0..3 for auth.  It may not rely on Rust enum variants for
these direct integer uses.  This repair is source-established but was not
applied in this resolution-only pass.

### RUST-001 — non-FFI Rust strings

**Accepted; same required controlled requeue repair as P1.**

The review's representation diagnosis is confirmed by the source evidence
above.  No counterevidence exists: the UAPI macros are literals with a trailing
NUL, while the candidate declares `&str`.  The required reimplementation is
the NUL-preserving, use-boundary mapping stated for P1.

### RUST-002 — named C enum constants lost integer-expression semantics

**Accepted; same required controlled requeue repair as P2.**

The candidate cannot satisfy the direct `int` comparisons, assignments, and
switch labels by exposing only Rust enum variants.  The top-level primitive
integer constants stated for P2 are required.  This does not establish the
ABI of the three C enum *types*.

### RUST-003 — named enum ABI is unproven

**Upheld; terminal blocker.**

The header gives the three named enum declarations (`handshake.h:13-30`) but
no fixed underlying type, layout, alignment, packed/transparent annotation,
or typed UAPI struct/function boundary.  A complete direct-use search finds
the enum tags only in this header; the selected consumers use the enumerator
values through `int` fields rather than materializing a named enum object.

Accordingly, the frozen ABI rows named above leave both layout and alignment
as `PENDING_REVIEW` for x86_64 and aarch64.  The captured Kbuild commands in
`rewrite/FILE_MAP.tsv` select the frozen compilers and targets but supply no
source-level fact that settles these type records.  Phase 1 prohibits a
compiler/layout probe, and `#[repr(C)]` is not source evidence for C enum
layout or for accepting only listed discriminants.  Replacing the types with
`i32`, retaining `#[repr(C)]` Rust enums, or otherwise selecting an enum
representation would therefore be an unreviewed design choice.

The proposed semantic closure is rejected: it changes these ABI and lifetime
records from `PENDING_REVIEW` to `COMPLETE` without the missing evidence, and
it is also bound to candidate SHA-256
`625f7e5ef1899949aca9e690e37ded57fd0029f5b2a1d9d5f6458e53b6fee4bb`, not the
current candidate SHA listed above.

## Required queue disposition (not performed here)

Enter `APPLYING` through the queue tool for the completed review stage, then
mark **BLOCKED** through that tool with the RUST-003 evidence above.  Do not
requeue merely to repair P1/P2: after those repairs the unresolved named-enum
ABI would still prevent closing every required semantic record and thus still
forbid `DONE`.
