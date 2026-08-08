# Resolution — S000779, attempt 1, P02

Applier: `gpt-5.6-terra` / `high`

I independently reopened the complete pinned
`arch/x86/include/uapi/asm/ldt.h`, the direct selected consumers
`arch/x86/include/asm/desc.h`, `arch/x86/kernel/tls.c`, and
`arch/x86/kernel/ldt.c`, the sealed candidate and snapshot, both review
reports and their semantic attestations, and the S000779 frozen records.  No
compiler, formatter, analyzer, test, runtime command, historical Lupos Rust
source, or candidate source edit was used.

## Dispositions

### Parity P1 — `struct user_desc` ABI and bit-field layout: BLOCKED

Confirmed.  The pinned header declares three `unsigned int` objects followed
by named `unsigned int` bit-fields, including the selected x86_64 `lm` field
(`ldt.h:21-41`).  The candidate instead makes a fourth ordinary `u32` field
and assumes its allocation order and masks.  `#[repr(C)]` establishes only
the Rust ordinary-field layout; it does not establish the frozen C compiler's
bit-field allocation unit, padding, or bit numbering.  The frozen `ABI.tsv`
row for `struct user_desc` retains `layout`, `alignment`, and `export_kind` as
`PENDING_REVIEW`; no source-backed target ABI record fills that gap.

This is user-visible rather than merely internal: `tls.c:119-123` and
`ldt.c:583-601` copy the complete object between userspace and the kernel,
then consume its fields.  Accepting the candidate's 16-byte low-bit-first
projection would therefore guess at bytes accepted by `set_thread_area`,
`get_thread_area`, and `modify_ldt`.  No source-only exact representation is
available in the frozen evidence.

Affected records remain unresolved: `SC1-5463f93831a07b738772ddea3cf73bbd3ccf028e90eff96748d8ada5bc823af8`,
`SC1-8655eb0964f26c453ddae4812fb5445d0ee7e8bf4e7c15def729fa8fdf2bf4c7`,
`SC1-0d37fac5188ba4328a6b44822aca714b3d412f146d4f82c6c358b8524cd8ded9`,
`SC1-018e330573a032ad63590eaa3d89ae8c82feee1d96b4ddc9884973b4eb71b315`,
`SC1-68a4c058f17c04256cfb6a8d8ef223a5fa95ba6665237b6b3a9c35e860fb37e7`,
and `SC1-77b2744a7b0a1f2caa7ae90bff78b460bef486f01255b8afe9b89ce95ecbe7d8`.

### Parity P1 — named field read/write and object-copy behavior: BLOCKED

Confirmed in material part.  Individual C bit-fields are lvalues but cannot
be addressed with `&`; the reviews' phrase "address-taking" is not relied on.
Their named read/write semantics are nevertheless required.  `desc.h:16-42`
reads the fields to populate a descriptor; `tls.c:53-80` branches on them;
and `tls.c:198-215` writes every selected field before `copy_to_user`.
The candidate offers only getters over a replacement `flags` word and no
source-backed write mechanism that can preserve the underlying UAPI bytes.
The raw-copy consumers make an independently invented setter API insufficient
without the unresolved ABI disposition above.

Affected records remain unresolved: `SC1-5463f93831a07b738772ddea3cf73bbd3ccf028e90eff96748d8ada5bc823af8`,
`SC1-8655eb0964f26c453ddae4812fb5445d0ee7e8bf4e7c15def729fa8fdf2bf4c7`,
`SC1-68a4c058f17c04256cfb6a8d8ef223a5fa95ba6665237b6b3a9c35e860fb37e7`,
`SC1-9e58c1a8da3e3437358247ce5f6b99d02d92dd90f52914ee01977b7e8eab8593`,
and `SC1-d4fde54277313a869f114481dd51288d7c59d271f0fbb7afda7fb6f0f1432123`.

### Parity P1 — macro, assembler, and conditional contract: BLOCKED

Confirmed.  `LDT_ENTRIES` and `LDT_ENTRY_SIZE` are outside the source's
`#ifndef __ASSEMBLER__` branch (`ldt.h:11-15`), while `user_desc` and the
three `MODIFY_LDT_CONTENTS_*` macros are inside it and `lm` is selected by
`__x86_64__` (`ldt.h:15-47`).  The candidate's Rust-only `u32` constants and
module boundary do not establish an equivalent C/assembly include, header
guard, or conditional interface.  Pinned consumers use the unsuffixed macro
expressions in arithmetic, including `ldt.c:154-162,515-520,632-634`,
`desc.h:201`, and `enlighten_pv.c:500,520`.  The frozen `SYMBOLS.tsv` rows
for every one of these five macros explicitly remain `PENDING_REVIEW`; they
provide no reviewed Rust constant-type/expression bridge.  A fixed `u32` is
therefore not source-proven parity.

Affected records remain unresolved: `SC1-0e171631b0e3a66312ad9301bf41277811af4233699fc82a852a9ae96c1d28c0`,
`SC1-24abe370ff799f25675f75d5ba1b5a87cd69f27aec81c766cfe179de5c6fa568`,
`SC1-40223b5dc6dad75dc4e0ccc0c1b822578a288ea37c192a8de78300f0c5916c23`,
`SC1-b1fd82ed4a565fbb32911df03e8d00459883007ac597145d335fbd9525750ad0`,
`SC1-ef3f1dd191376f21f327ad6c132d24dd116347a726294694061f2562df9dd7eb`,
`SC1-b708e35e1381016a83f168ec4b3a1518915b746cad2920db2b66e65d1797bccc`,
`SC1-c55dc368b4e818f1f0c7dd2365c8fe91cebd8af95598246f1f17196feaf09638`,
`SC1-74ff04f8e426b841bc3ea99d9da31029992b3e44ed03734de7708232593c0c22`,
and `SC1-f008baa1168242bd5126b6dc743d552ad5726c4998b11da9688e6c5199e209bb`.

### Rust R1 — replacement `flags` interface: BLOCKED

Confirmed for field reads and writes; corrected only the nonessential
address-taking characterization noted above.  The candidate does not map the
source's fieldwise mutations in `tls.c:198-215`, nor does it provide a
source-proven conversion between those fields and the copied UAPI object.
The sealed candidate cannot be accepted without changing the ABI mechanism,
and the frozen records cannot prove such a change exact.

### Rust R2 — substituted bit-field ABI: BLOCKED

Confirmed.  The pinned header gives declarations but not the target C
bit-field allocation convention.  The frozen ABI row retains all material
layout values as `PENDING_REVIEW`; the same unresolved records listed for
Parity P1 remain open.  Accepting masks and a raw `u32` allocation word would
be an unsupported ABI decision.

### Rust R3 — C unsuffixed macros changed to `u32`: BLOCKED

Confirmed.  The candidate fixes all five macro values to `u32`, while the
pinned header gives unsuffixed integer replacement lists.  The frozen macro
records listed under Parity P1 do not establish that type conversion or an
equivalent C-preprocessor/FFI boundary.  The selected arithmetic consumers
make the expression contract material, so no semantic closure can mark the
records complete from the available source evidence.

## Result

`BLOCKED`.  Exact source parity cannot be established for the user-visible C
bit-field layout and associated macro/preprocessor contract.  The candidate
was not edited, no final semantic closure was prepared or committed, and no
`DONE` transition is warranted.
