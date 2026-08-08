# Rust review — S000779 attempt 1, P02

Reviewer role: `rust_reviewer`  
Model/effort: `gpt-5.6-terra` / `high`  
Scope inspected: pinned `arch/x86/include/uapi/asm/ldt.h`, the candidate
snapshot, direct frozen records, and pinned x86 consumers. No compiler,
formatter, analyzer, test, runtime, or historical Lupos source was used.

## Findings

### R1 — Blocker: the replacement `flags` field is not the C UAPI member interface

Pinned `ldt.h:24-38` declares seven named C bit-fields plus the x86_64 `lm`
bit-field. They are addressable lvalue members at their individual C language
names. The candidate instead declares one public `flags: u32` field and only
read-only methods such as `seg_32bit(&self)`. This is not a faithful
source/FFI-facing representation: it removes the named member interface and
cannot express the writes performed by the pinned consumers. In particular,
`arch/x86/kernel/tls.c:198-213` assigns every named bit-field while filling a
`user_desc`; `arch/x86/include/asm/desc.h:16-41` and `tls.c:33-80` directly
read those members. A Rust translation must provide an exact reviewed mechanism
for both the UAPI storage and the selected read/write users; accessor-only
replacement changes the interface and mutation semantics.

The issue is not a stylistic naming question: `struct user_desc` crosses the
modify_ldt/TLS user ABI, is copied as an object, and is used by selected core
code. Do not approve the candidate until the source-defined field access and
write behavior is mapped without inventing a private replacement ABI.

Affected semantic records:
`SC1-68a4c058f17c04256cfb6a8d8ef223a5fa95ba6665237b6b3a9c35e860fb37e7`,
`SC1-0d37fac5188ba4328a6b44822aca714b3d412f146d4f82c6c358b8524cd8ded9`,
`SC1-018e330573a032ad63590eaa3d89ae8c82feee1d96b4ddc9884973b4eb71b315`.

### R2 — Blocker: no permitted evidence establishes the substituted bit-field ABI

`#[repr(C)]` on four `u32` members does not by itself establish that Rust's
`flags` word represents the C implementation's unsigned-int bit-field storage,
allocation-unit placement, padding, or bit numbering. The candidate assumes
least-significant-bit first ordering and a 16-byte, four-word representation.
Those properties are material because the C object is copied to/from userspace
and because the selected consumers operate on its fields. The direct frozen
ABI record for `struct user_desc` is still `PENDING_REVIEW`; it supplies no
size, alignment, bit-field allocation, padding, or exported UAPI layout proof.
The header itself also does not define the bit-field allocation convention.

An x86 target-specific ABI/source record must establish the exact layout and
access mechanism before a final semantic closure can claim it `COMPLETE`.
Absent that evidence, accepting the manually assumed masks/positions would be
guessing about a user-visible ABI.

Affected semantic records:
`SC1-68a4c058f17c04256cfb6a8d8ef223a5fa95ba6665237b6b3a9c35e860fb37e7`,
`SC1-0d37fac5188ba4328a6b44822aca714b3d412f146d4f82c6c358b8524cd8ded9`.

### R3 — Finding: UAPI macros changed from C unsuffixed integer expressions to `u32`

All five source macros are unsuffixed C integer constants. The candidate fixes
their Rust type to `u32`. That changes contextual comparison, promotion, and
FFI argument behavior relative to the C macro expressions, which have `int`
type on the frozen x86 C environment. The frozen macro records remain pending;
they do not establish a source-reviewed C-to-Rust public-constant type mapping.
The applier must either provide direct frozen evidence for the intended API
types or retain this as an unresolved semantic item.

Affected semantic records:
`SC1-ef3f1dd191376f21f327ad6c132d24dd116347a726294694061f2562df9dd7eb`,
`SC1-b708e35e1381016a83f168ec4b3a1518915b746cad2920db2b66e65d1797bccc`,
`SC1-c55dc368b4e818f1f0c7dd2365c8fe91cebd8af95598246f1f17196feaf09638`,
`SC1-74ff04f8e426b841bc3ea99d9da31029992b3e44ed03734de7708232593c0c22`,
`SC1-f008baa1168242bd5126b6dc743d552ad5726c4998b11da9688e6c5199e209bb`.

## Assessment

Reject. The candidate has no unsafe blocks, ownership, callback, Send/Sync,
Drop, or allocation behavior to approve; the critical unresolved issue is its
replacement of a C bit-field UAPI and the absent target-specific ABI proof.
