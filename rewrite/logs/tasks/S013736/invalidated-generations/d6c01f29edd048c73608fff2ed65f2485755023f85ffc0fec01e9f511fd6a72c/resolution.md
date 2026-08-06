# Resolution — S013736

**Disposition: BLOCKED.** The candidate is rejected as a complete
translation. No source change is accepted for this one-file task because the
selected `SPMI_MODULE_PREFIX` source contract cannot be expressed by the
candidate's Rust macro, and no frozen cross-language macro representation is
provided.

## P1 — `SPMI_MODULE_PREFIX` literal composition and array semantics

**Accepted.** `include/linux/device-id/spmi.h:10` defines an object-like C
preprocessor macro whose replacement list is the string-literal token
`"spmi:"`. At every C expansion site, it may be immediately adjacent to other
string-literal tokens; C combines those tokens before the single terminating
NUL is formed. The result remains a string-literal array, so array-sensitive
contexts such as `sizeof(SPMI_MODULE_PREFIX "device")`, and pointer-decay
contexts, retain their respective C semantics.

The candidate's function-like `SPMI_MODULE_PREFIX!()` expands to the already
NUL-terminated Rust byte-string expression `b"spmi:\\0"`. It cannot be used
without a Rust macro delimiter, cannot compose with an adjacent caller literal,
has an explicit NUL before any potential suffix, and yields a reference rather
than the C macro's per-use string-literal array. Its comment therefore makes an
incorrect equivalence claim.

Neither Rust declarative macros nor a Rust constant can act as a delimiter-free
object-like token replacement. A macro that instead accepts a suffix, or a
`concat!`/byte-slice helper, would require rewritten caller syntax and changes
the array, `sizeof`, pointer-decay, and token-composition interface. It is a
new convenience API, not a faithful translation. The absence of an observed
in-tree use beyond the definition does not remove this selected operative
macro's public header contract.

## Independently confirmed source contracts

The frozen common task selects the `__KERNEL__` branch for both x86_64 and
AArch64. Within that selected branch, `kernel_ulong_t` is C `unsigned long`;
the frozen targets are LP64. `SPMI_NAME_SIZE` is the C `int` literal `32`, and
the source aggregate is a fixed 32-element C `char` array followed by
`kernel_ulong_t`. The frozen command evidence records unsigned C `char`, so
the candidate's `[u8; 32]` and `#[repr(C)]` direction is consistent with the
field-byte representation. These facts do not resolve P1.

## Blocking condition

The pending semantic records for this task cannot be closed while an operative
source macro lacks a faithful representation. Reopen the applicable
scope/source mapping and provide an audited representation for object-like C
string-literal token composition, including its NUL, array/`sizeof`, and
pointer-decay contracts at every translated consumer. Alternatively, establish
from the frozen selection evidence that the macro is not an operative required
contract and update the frozen manifests before requeueing. Do not substitute a
function-like macro, `const`, byte slice, or helper API.

No compiler, formatter, analyzer, linker, test, runtime, debugger, or build
command was run.
