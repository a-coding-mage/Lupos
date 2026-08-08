# S012620 implementation — attempt 4

Source reviewed: `vendor/linux/include/crypto/dh.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, with
`vendor/linux/crypto/dh_helper.c` read for the declared helpers' raw-pointer
and aliasing contracts.

`src/include/crypto/dh.rs` is a fresh path-preserving declaration translation:

- `struct dh` retains C field order, three `const void *` fields, and three
  `unsigned int` fields as `*const c_void` and `u32`, under `#[repr(C)]`.
- The structure is `Copy, Clone`, matching the C structure's ordinary
  copyable-value behavior.
- All four header declarations retain their C names, raw-pointer signatures,
  and C ABI. The extern declarations document the readable/writable regions
  and, for both decoders, the source-proven result-field aliases into `buf`.

No implementation body belongs in this header task. No compilation, formatter,
test, linker, or runtime command was run.
