# Applier resolution — S016417 / attempt 1

Outcome: **BLOCKED — do not mark DONE or requeue on the present evidence.**

The applier reopened the complete pinned
`vendor/linux/include/uapi/linux/thermal.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate,
implementation record, both review reports, the S016417 queue/scope/symbol/ABI
and lifetime rows, and the directly relevant pinned generic-netlink context.
No compiler, formatter, linker, test, runtime tool, or historical Rust source
was used.

## Finding dispositions

### F1 — C string-literal macro representation

**Disposition: ACCEPTED IN PART; candidate change required; unresolved.**

`thermal.h:22,24-25` defines the three names as C string-literal replacement
lists.  The current `&str` values do not retain the trailing NUL byte and do
not provide a C character-array representation.  Thus they cannot preserve
the source contract of the three selected macros.

The review's claimed direct `char *` initializer is not supported by the
pinned context and is corrected here: `include/net/genetlink.h:29-32` and
`:78-81` declare `genl_multicast_group.name` and `genl_family.name` as
`char name[GENL_NAMSIZ]`.  `drivers/thermal/thermal_netlink.c:19-20,906`
initializes those arrays with the macro-expanded literals; this is C array
initialization, not initialization of a `char *` field.  That correction does
not cure the candidate: the literal bytes, including NUL, and the array-use
contract are still absent.

Any renewed candidate would need an explicit Rust representation of the exact
NUL-terminated literal byte arrays and an independently reviewed mapping for
each C array-initialization use.  No such corrected candidate is sealed here.

### F2 — unscoped public enumerator names

**Disposition: ACCEPTED; candidate change required; unresolved.**

The pinned declarations at `thermal.h:9-12,14-19,28-58,61-64,68-90,94-107`
introduce the selected enumerators as header-scope C identifiers.  The frozen
S016417 `SYMBOLS.tsv` rows select every enumerator for both architectures,
including each `__THERMAL_GENL_*_MAX` sentinel.  The candidate exposes them
only as type-qualified Rust enum variants, so it supplies no equivalent
module-scope Linux-named integer constants.  The bare uses in
`drivers/thermal/thermal_core.c:229,233,628,634` corroborate the required
unscoped expression contract.

The source supports recreating the names and their listed ordinal values, but
not accepting the current representation: their carrier type/ABI remains
unresolved as described under RUST-002.

### RUST-001 — string macro UAPI/FFI contract

**Disposition: ACCEPTED IN PART; candidate change required; unresolved.**

The NUL-byte and non-`&str` conclusions are accepted for the same pinned
macro evidence as F1.  The asserted pointer-decay use at the cited direct
netlink initializers is rejected: those initializers target the fixed arrays
shown above, so they copy literal characters into the fields.  This does not
weaken the finding.  A Rust `&str` is neither the literal byte array nor an
array initializer and omits the required terminator.

The exact byte arrays can be specified from `thermal.h`, but the source-only
record has not established a reviewed Rust mapping that preserves every
selected C macro expression context.  The sealed candidate therefore cannot
close this finding.

### RUST-002 — fieldless Rust enum validity and ABI

**Disposition: ACCEPTED; blocking source-evidence gap.**

The six C enum declarations provide named integer constants, but the pinned
header supplies no fixed enum storage width, signedness, alignment, or Rust
validity domain.  The frozen `ABI.tsv` rows for all six types on both x86_64
and AArch64 still record layout, alignment, and linkage as `PENDING_REVIEW`.
The matching `LIFETIMES.tsv` rows also remain `PENDING_REVIEW`; the current
fieldless `#[repr(C)]` enums do not close either set of records.  In
particular, a Rust fieldless enum excludes unlisted discriminants, whereas
the source record does not establish that only the listed values may inhabit
the C carrier.

Replacing the enums with `c_int` aliases and module-scope constants would be
a new ABI decision, not a source-proven correction: neither the pinned header
nor the frozen ABI record establishes that `c_int` is the exact enum carrier
for every selected declaration on both architectures.  Selecting it would
therefore guess at an unresolved ABI and violate the Phase 1 source-evidence
gate.

## Closure and queue recommendation

This source-only record cannot truthfully establish the enum ABI necessary to
recreate and review a corrected candidate.  The candidate also still needs
the string-macro and unscoped-enumerator corrections, which would invalidate
the sealed candidate snapshot and require fresh independent reviews.  Under
the task instruction, the appropriate queue action is **BLOCKED**, with the
concrete reason: `pinned source and frozen ABI records do not establish the
six C enum carrier ABI for x86_64 and aarch64; current candidate additionally
omits NUL-byte macro arrays and unscoped enumerator constants`.

No source, evidence file other than this resolution, or queue state was
changed by this adjudication.
