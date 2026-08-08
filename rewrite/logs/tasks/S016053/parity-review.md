# Parity review — S016053, attempt 1, slot 1

Reviewer: parity reviewer, P01 slot 1 (manual source inspection only; no
compiler, formatter, linker, test, rust-analyzer diagnostics, or historical
Lupos source was used).

Reviewed candidate: `src/include/uapi/linux/arm_sdei.rs`.
Pinned source: `vendor/linux/include/uapi/linux/arm_sdei.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.
Frozen task scope: `S016053`, aarch64, `RUST_TRANSLATE` (`rewrite/SCOPE.tsv`,
row 16054).  The sealed task-local semantic proposal has 107 records and is
bound to the current candidate (`semantic-closure-proposal.sha256`, proposal
digest `f17df42edc770a0a927beb21e01746f6e31f96c2fbf7184df252eed5e8614ccd`).

## Result: FINDINGS — reject current candidate and its proposed semantic closure

### PARITY-001 — Linux scalar macro types/widths were changed

Linux symbols: `SDEI_VERSION_MAJOR_SHIFT`, `SDEI_VERSION_MAJOR_MASK`,
`SDEI_VERSION_MINOR_SHIFT`, `SDEI_VERSION_MINOR_MASK`,
`SDEI_VERSION_VENDOR_SHIFT`, `SDEI_VERSION_VENDOR_MASK`,
`SDEI_EVENT_REGISTER_RM_ANY`, `SDEI_EVENT_REGISTER_RM_PE`,
`SDEI_EVENT_STATUS_RUNNING`, `SDEI_EVENT_STATUS_ENABLED`,
`SDEI_EVENT_STATUS_REGISTERED`, `SDEI_EV_HANDLED`, `SDEI_EV_FAILED`,
`SDEI_EVENT_INFO_EV_TYPE`, `SDEI_EVENT_INFO_EV_SIGNALED`,
`SDEI_EVENT_INFO_EV_PRIORITY`, `SDEI_EVENT_INFO_EV_ROUTING_MODE`,
`SDEI_EVENT_INFO_EV_ROUTING_AFF`, `SDEI_EVENT_TYPE_PRIVATE`,
`SDEI_EVENT_TYPE_SHARED`, `SDEI_EVENT_PRIORITY_NORMAL`, and
`SDEI_EVENT_PRIORITY_CRITICAL`.

Local evidence: the pinned header defines all of those values as unsuffixed
decimal/hexadecimal C integer constants at lines 28–33 and 48–71.  On the
frozen AArch64 C ABI, decimal literals and `0x7fff`/`0xffff` are `int`, while
`0xffffffff` is `unsigned int`.  The candidate changes the shifts and all
positive selector/status/type/priority values to `u32`, and changes all three
masks to `u64` (candidate lines 36–41 and 71–90).  The C types are material:
they control the usual arithmetic conversions in the expression macros and at
the `unsigned long` firmware-call boundary.  For example,
`sdei_to_linux_errno(unsigned long sdei_err)` in the pinned
`drivers/firmware/arm_sdei.c` switches on these macros (lines 119–135), and
the same C file passes selector macros to `unsigned long` arguments (lines
186–193 and 217–223).  Replacing signed `int` constants with `u32`, or an
`unsigned int` mask with `u64`, changes widths, signs, and expression typing;
it is not a type-neutral spelling change.

The affected `SYMBOLS.tsv` `selection_expression` closure records must not be
accepted as `COMPLETE` until the Rust representation preserves the pinned C
types/conversions at every translated use (or the task is blocked if that
cannot be expressed exactly).

### PARITY-002 — `SDEI_1_0_FN` loses the source unsigned-arithmetic mechanism

Linux symbol: `SDEI_1_0_FN`.

Local evidence: pinned header line 8 defines
`(SDEI_1_0_FN_BASE + (n))`; `SDEI_1_0_FN_BASE` at line 6 is the C
`unsigned int` hexadecimal constant `0xC4000020`.  Thus C usual arithmetic
conversions and well-defined unsigned wrap apply to this macro.  The candidate
macro at lines 12–15 instead hardcodes an unsuffixed literal and uses ordinary
Rust `+`, bypassing `SDEI_1_0_FN_BASE` entirely.  Its type is inferred from
the caller, and an overflowing `u32` invocation has Rust overflow behavior
rather than the source macro's defined modulo-2^32 result.  This is a
mechanism, type, and overflow-semantics change even though the seventeen
currently materialized function-ID constants use small operands.

The proposed completion for `SDEI_1_0_FN` is therefore unsupported.  Resolve
the representation against the pinned caller operand types and preserve the
source expression's named-base and unsigned-conversion behavior; do not rely
on the current constants alone.

### PARITY-003 — selected UAPI include-guard macro has no evidenced mapping

Linux symbol: `_UAPI_LINUX_ARM_SDEI_H`.

Local evidence: the pinned source opens the selected guard at lines 3–4 and
closes it at line 73.  Frozen `SYMBOLS.tsv` row 351509 inventories the guard
as an operative macro, but the candidate has no corresponding guard or an
explicit Rust module-single-inclusion mapping (candidate lines 1–90).  Its
task-local proposal nevertheless changes the guard's
`selection_expression`/`status` records to `SOURCE_REVIEWED_VALUE`/`COMPLETE`
without citing a candidate mechanism.  The candidate also begins with C block
comments rather than the required immutable `// SPDX...` provenance form, so
the claimed Rust module mapping cannot be inferred from the mandatory source
header either.

The applier must provide a source-backed, explicit mapping for the guard and
correct the required provenance form before this selected macro can be closed.

## Coverage notes

All remaining named function-ID constants, version extractor names, SDEI error
values, and selector/status/type/priority names were manually compared against
pinned header lines 6–71; their numeric spellings are present.  This does not
resolve the type, overflow, and guard findings above.  The header has no
functions, structs, storage, locks, refcounts, allocation paths, or branding
delta to accept.  `rewrite/BRANDING_ALLOWLIST.tsv` contains no applicable
authorization for a name change, and none was found.

No compiler or runtime evidence was used.
