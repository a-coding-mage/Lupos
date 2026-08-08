# Rust source review — S016228 (slot 2)

Reviewed the current candidate only: `src/include/uapi/linux/lockd_netlink.rs`
(`9fc0bf7a2b8bf1ed4f35dfae9d48aaa610d7bafcf0c6f98e22967d5b6a579a33`),
the sealed current proposal
(`4b2d187beee6deb1fa4ff18c1d6e329fb672024c71e179ea746a219ba1855395`),
and pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
No compiler, formatter, test, rust-analyzer, historical Rust source, or other
task evidence was used.

## Findings

### RUST-001 — `LOCKD_FAMILY_NAME` changes the C literal/array contract (high)

Pinned `include/uapi/linux/lockd_netlink.h:10` defines the macro as the C
string literal `"lockd"`.  Its direct consumer,
`fs/lockd/netlink.c:37-44`, initializes `struct genl_family.name` with that
macro.  The field is a fixed `char name[GENL_NAMSIZ]` array
(`include/net/genetlink.h:78-82`; `GENL_NAMSIZ` is 16), so the C initializer
copies the five characters and terminator into the 16-byte field and
zero-initializes the remaining bytes.

The candidate instead publishes a `&[u8; 6]`: a Rust reference to an unsigned
byte array, not a C `char` string-literal expression or an array initializer.
It requires an explicit conversion at every native/FFI use, cannot itself
initialize the 16-byte field, and carries Rust-reference provenance rather
than the original literal-array expression.  Its NUL byte is correct but does
not recover the lost initializer and element-type behavior.  The applier must
use the project’s C-character-compatible representation and ensure the
translated `genl_family.name` initialization retains exactly the C copy and
zero-fill behavior; this candidate cannot be accepted as that representation.

Affected semantic keys:

- `SC1-5376d0cac64bf862d5c3371e1bf3658e801935356c1bbdc9abd0124ed91ddebc`
- `SC1-f3bca7eb88f8c64c63a899011551332b36734e50b28537b35cd5d7f7c61654c6`
- `SC1-7cb16858b01562e44f3e06e4bd4ff3b9df5e1d32cbcf6e34106e6274ea9f3132`
- `SC1-5e5d1b5a6993bd0f90fdb5971c839172a39ee02da955fa4f55a65ba4f598850e`

### RUST-002 — selected C header-guard behavior is unrepresented (medium)

Pinned header lines 7-8 and 30 implement the selected
`_UAPI_LINUX_LOCKD_NETLINK_H` preprocessor guard.  The candidate declares no
equivalent compile-time guard or documented source-level mechanism.  A Rust
module's ordinary item namespace does not reproduce the C macro's observable
include-state behavior for repeated textual inclusion or conditional checks.
The proposal therefore cannot mark this operative macro complete without a
specific, source-supported mapping.  The applier must establish and document
an exact Rust-side equivalent, or block the task rather than silently erase
the macro behavior.

Affected semantic keys:

- `SC1-15a57dc865035ddc992a081fce1b5b2dcda1adb45cad7ffd51908ad9c431f1e3`
- `SC1-082270722905aec1818256d3c2b6fe01e09e5081475df63a8705643d411f8fd6`
- `SC1-b0440b5118b82cb0466b5eb601b0a7982da234528986d4e4892716722cedb582`
- `SC1-ffebb9a4c2c542be7876e55d4c384b2aaa8c7103c02aab71fe93e97e817094f1`

## Checked without separate findings

The two anonymous C enums introduce no named ABI type in this declaration;
their constants are `int` values on both frozen ABIs.  The candidate's `i32`
constants preserve every enumerator value, sequence, and derived maximum
(1/2/3/4/3 and 1/2/3/2 respectively), without adding a Rust enum whose layout
or namespace would differ.  There are no allocations, callbacks, mutable
state, borrows crossing calls, `unsafe` blocks, FFI declarations, `Drop`, or
`Send`/`Sync` assertions to validate in this file.  The only FFI/provenance
concern is RUST-001's reference substitution.

## Disposition

FINDINGS.  Do not advance this candidate to application until both findings
are resolved from pinned source evidence.
