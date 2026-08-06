# S012620 implementation

Translated `include/crypto/dh.h` to `src/include/crypto/dh.rs` from pinned
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` for AArch64.

The C layout is preserved with `#[repr(C)]`: three `const void *` fields in
source order followed by three `unsigned int` length fields.  The declarations
retain C linkage and raw-pointer contracts.  In particular, successful decode
stores borrowed pointers into the caller's packet buffer; neither the structure
nor these declarations own or free that storage.

Context inspected: the complete pinned header plus `crypto/dh.c` and
`crypto/dh_helper.c`, which define the packet encoding and establish the
decode-aliasing contract.  No build, formatter, test, or runtime command ran.
