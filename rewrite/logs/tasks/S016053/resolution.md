# Resolution — S016053, attempt 1, P01

## Result: BLOCKED

The candidate cannot be accepted and the task cannot reach `DONE`.  Manual
inspection establishes the C definitions, but the frozen records do not
establish an exact Rust mapping for this UAPI header's function-like macro
integer domain, C usual-arithmetic conversions, or header-scoped macro
visibility.  Inventing a narrowing `u32` API, a conversion trait, or a
crate-root export would be a new unreviewed design.  No source or queue change
is made by this resolution-only application pass.

Pinned source: `vendor/linux/include/uapi/linux/arm_sdei.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Relevant frozen evidence:

- `rewrite/SYMBOLS.tsv` keeps every S016053 conditional and operative macro,
  including `_UAPI_LINUX_ARM_SDEI_H` and `SDEI_1_0_FN`, as `PENDING_REVIEW`.
- `rewrite/ABI.tsv` contains no S016053 row defining a transformed macro
  interface; `rewrite/PORTING.md:20-23` requires those pending semantic facts
  to be resolved from pinned context before `DONE`.
- `rewrite/configs/aarch64/frozen.config` selects
  `CONFIG_ARM_SDE_INTERFACE=y`; the header is consumed through
  `include/linux/arm_sdei.h`, and the selected kernel objects include
  `arch/arm64/kernel/sdei.o` and `drivers/firmware/arm_sdei.o`.

## Finding dispositions

### PARITY-001 — BLOCKED

Accepted.  `arm_sdei.h:28-33,48-71` supplies C macro tokens, not Rust-typed
constants.  On the frozen AArch64 C model the decimal literals and
`0x7fff`/`0xffff` are `int`, whereas `0xffffffff` is `unsigned int`; their
types then participate in C's usual arithmetic conversions.  The candidate
publishes fixed `u32` and `u64` constants instead.  The only direct in-tree
version-extractor caller is `drivers/firmware/arm_sdei.c:975-978`, where the
operand is `u64`, but that use does not establish a representation for the
selected public macros at other valid C operand types.  No ABI or porting
record supplies the missing Rust conversion contract.  The candidate's
proposed `COMPLETE` semantic records therefore remain unsupported.

### PARITY-002 — BLOCKED

Accepted.  `arm_sdei.h:6-8` defines `SDEI_1_0_FN(n)` as
`SDEI_1_0_FN_BASE + (n)`, with an `unsigned int` base, so its result and
overflow behavior depend on the argument's C type after the usual arithmetic
conversions.  The candidate hardcodes the literal, does not use
`SDEI_1_0_FN_BASE`, and uses Rust `+`; it provides neither the C conversions
nor defined modulo-2^32 arithmetic for the `unsigned int` case.  Searching
the complete pinned C/header source finds only this header's seventeen literal
macro expansions, but that does not narrow the selected function-like UAPI
macro's defined argument surface.  A `wrapping_add(u32)` replacement would
silently narrow wider C operands, while a generic conversion mechanism has no
frozen ABI/porting evidence.  Exact parity is therefore not established.

### PARITY-003 — ACCEPTED; correction required, not independently blocking

Accepted in part.  The candidate violates the mandatory Rust provenance form:
it begins with a C block SPDX comment rather than the required `// SPDX...`
line.  That is a mechanical source correction.  The C guard at
`arm_sdei.h:3-4,73` is a preprocessor single-inclusion mechanism.  A Rust
module's single compilation can be its analogue only when the generated module
tree supplies and records that mapping; no such task-local mapping exists yet,
and the candidate cannot claim the guard's semantic record `COMPLETE` now.
This finding does not cure the independent macro-domain block above.

### RUST-S016053-01 — BLOCKED

Accepted; same unresolved semantic record as PARITY-002.  The candidate's
ordinary Rust addition is profile-dependent on overflow and its unconstrained
macro argument has no source-backed C-conversion mapping.  No local pinned
caller establishes a safe restriction of the UAPI macro to `u32`.

### RUST-S016053-02 — BLOCKED

Accepted; same unresolved semantic-record family as PARITY-001.  The source
and direct callers establish literal spellings and the `u64` version-word use,
but not a faithful fixed Rust type surface for all selected positive macros.
The absence of an S016053 ABI decision prevents accepting the candidate's
`u32`/`u64` choices.

### RUST-S016053-03 — BLOCKED

Accepted.  `#[macro_export]` puts each candidate macro at crate root, whereas
the pinned definitions are visible only to C translation units that include
this header.  The frozen ABI has no macro visibility or collision decision,
and no deterministic module index yet exists for this file.  Removing the
attributes, re-exporting them, or replacing the macros with functions would
each choose a Rust interface not established by the pinned source and frozen
records.  This cannot be closed by style preference.

## Required later evidence

To unblock, the workflow needs a frozen, source-backed mapping that specifies
the Rust surface for C function-like UAPI macros: supported C operand domains,
the exact usual-arithmetic-conversion and overflow rule, one-evaluation
behavior, and header/module visibility.  It must also define the guard's
module-inclusion mapping.  Until that authority exists, the task must remain
or be transitioned to `BLOCKED`; it must not be marked `DONE` and its 107
proposed semantic completions must not be applied.
