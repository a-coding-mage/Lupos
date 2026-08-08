# S016371 parity review — FINDINGS

Scope reviewed: pinned `include/uapi/linux/seg6_genl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, fresh candidate
`src/include/uapi/linux/seg6_genl.rs`, its candidate diff, frozen S016371
task/manifest records, sealed semantic proposal, and direct SEG6/generic-netlink
consumer context.  No compiler, formatter, test, or diagnostic was invoked.

Frozen bindings reviewed: Phase 0 identity
`0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`;
queue fingerprint
`cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`;
scope, symbols, ABI, and lifetime hashes match the task-provided frozen values.

## Findings

### PARITY-001 — `SEG6_GENL_NAME`: no exact C-string/`genl_family.name` binding

Pinned source `vendor/linux/include/uapi/linux/seg6_genl.h:5` defines
`SEG6_GENL_NAME` as the C string-literal macro `"SEG6"`.  Direct consumer
`vendor/linux/net/ipv6/seg6.c:494-505` initializes
`struct genl_family.name` from that macro; `vendor/linux/include/net/genetlink.h:78-82`
defines that destination as `char name[GENL_NAMSIZ]`.  Thus the source value
has C string-literal storage/terminator semantics at this consumer boundary.

The candidate substitutes `pub const SEG6_GENL_NAME: &str = "SEG6";`.  An
`&str` is a Rust fat slice and contains no terminating NUL; the candidate also
contains no exact array or FFI conversion binding for the `char[GENL_NAMSIZ]`
consumer.  It consequently does not establish the required source-level UAPI
representation or initialization behavior.  No such binding may be inferred
from the numeric constants.  Restore an exact, source-evidenced binding (or
block the task until its owning boundary is specified) rather than treating
the `&str` as ABI-equivalent.

### PARITY-002 — `_UAPI_LINUX_SEG6_GENL_H`, `SEG6_GENL_*`,
`SEG6_ATTR_MAX`, and `SEG6_CMD_MAX`: C preprocessor contract is absent

Pinned header lines 2-3 and 33 supply the `_UAPI_LINUX_SEG6_GENL_H` include
guard.  Lines 5-6, 20, and 31 define C macros, including the token expressions
`(__SEG6_ATTR_MAX - 1)` and `(__SEG6_CMD_MAX - 1)`.  These names and their
preprocessor expansion/guard behavior are part of this UAPI header's consumer
contract.  The candidate provides Rust constants only and has no C-compatible
header, guard, macro export, or documented exact binding preserving that
contract.

The direct consumer confirms the values are operative rather than descriptive:
`seg6.c:140` uses `SEG6_ATTR_MAX + 1` for the policy array, and lines 494-505
use `SEG6_GENL_VERSION`, `SEG6_ATTR_MAX`, and
`SEG6_CMD_GET_TUNSRC + 1` in the registered family.  A Rust `const` can retain
the arithmetic value for Rust callers, but cannot itself provide the required
C macro behavior.  This is a missing source binding, not a justification to
guess a replacement interface.

## Verified portions that do not cure the findings

The candidate preserves both anonymous-enum value sequences: attributes
`SEG6_ATTR_UNSPEC..__SEG6_ATTR_MAX` are `0..8`, commands
`SEG6_CMD_UNSPEC..__SEG6_CMD_MAX` are `0..5`, and the derived maximum values
are respectively 7 and 4.  It also avoids incorrectly introducing a named
Rust enum: the pinned anonymous C enums place their enumerators in the C
ordinary identifier namespace, and the candidate's unqualified module
constants retain direct Rust-name access.  `SEG6_GENL_VERSION` remains 1.

Those correct values preserve the generic-netlink command/attribute numbers
used by `seg6.c` (including `.cmd` and `.resv_start_op` at lines 465-505), but
do not provide the missing C UAPI macro/string/guard boundary above.  No
unauthorized branding was found.

## Semantic-closure review

The sealed proposal marks the anonymous enum ABI/lifetime fields
`NOT_APPLICABLE` and the enumerator/macro selection records complete for both
architectures.  The anonymous-enum records themselves have no named C type to
export, and their proposed non-layout/non-lifetime dispositions are
source-consistent.  However, the proposal contains no field-level record for
the required C macro/header-guard/string boundary identified in PARITY-001 and
PARITY-002.  Its proposed `COMPLETE` status cannot close that unrepresented
missing binding.  This review therefore records semantic findings without
guessing a disposition for absent source evidence.
