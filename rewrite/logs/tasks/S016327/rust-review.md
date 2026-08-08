# Rust semantic review — S016327 / P02 / attempt 1

Result: **FINDINGS**

No compiler, formatter, analyzer, test, or runtime command was run. This review
read the pinned `include/uapi/linux/personality.h`, the candidate snapshot, and
the frozen SCOPE/SYMBOLS/ABI/LIFETIMES records plus direct pinned consumers.

## F1 — unresolved C enumeration ABI and integer-domain contract

The candidate replaces both anonymous C enum declarations with `pub const ...:
i32` items, and closes their ABI records as `SOURCE_REVIEWED_VALUE`. The pinned
source establishes the numeric values, but it does not establish the frozen
C-compiler's chosen ABI/layout/alignment/export treatment for either anonymous
enum on either approved target. Those exact fields are still
`PENDING_REVIEW` in the authoritative ABI manifest.

This is behaviorally material rather than merely cosmetic: the C enumerators
have C enumeration / `int` expression semantics and are consumed in expressions
whose other operand is `unsigned int` (`current->personality` and `per_clear`),
such as `current->personality & UNAME26`, `~PER_MASK`, and
`bprm->per_clear |= PER_CLEAR_ON_SETID`. C applies its integer promotions and
usual arithmetic conversions there. A Rust `i32` const has a fixed Rust type
and cannot participate in the corresponding `u32` operations without an
explicit, reviewed conversion at each consumer. The candidate provides neither
the target-specific C enum contract nor a source-proven Rust representation / FFI
boundary for it. The numeric literals are all representable in `i32`, and no
literal expression here overflows or shifts, but that fact does not resolve the
typed-expression and ABI contract.

The affected records are the layout, alignment, and export-kind closures for
`anonymous_enum@11` and `anonymous_enum@42` on x86_64 and AArch64. Do not mark
them complete without frozen source/toolchain evidence that establishes the
contract and an exact Rust mapping. This is a blocking finding under the
zero-difference rule.

## F2 — `PER_CLEAR_ON_SETID` has only been preserved as a value, not as its C expression contract

`PER_CLEAR_ON_SETID` is a C preprocessor macro whose replacement list is an
`int`-typed OR expression over enumerators. The candidate changes it into one
fixed `i32` Rust constant. Its evaluated bit pattern is correct, and it has no
side effects, shifts, or overflow in the pinned definition. But the frozen
manifest leaves its selected macro expression `PENDING_REVIEW` for both
targets, while the proposal closes it with an unspecified
`SOURCE_REVIEWED_VALUE`. The direct consumers OR that result into unsigned
fields. Exact parity requires an established Rust expression/type/conversion
rule for this macro and its consumers, not only a matching constant value.

No ownership, aliasing, `unsafe`, pinning, `Send`/`Sync`, allocation, callback,
Drop, or synchronization issue appears in this constants-only candidate; the
rejecting issues are the unresolved integer/ABI and macro-expression contracts.
