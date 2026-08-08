# Implementation evidence

- Task: `S016005`
- Attempt: `1`
- Pipeline: `P02`
- Linux source: `vendor/linux/include/uapi/asm-generic/hugetlb_encode.h`
- Destination: `src/include/uapi/asm-generic/hugetlb_encode.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by both frozen x86_64 and AArch64 configurations)

The complete pinned header was read. It contains only the include guard, the
shared huge-page flag shift and mask, and thirteen unsigned encoded-size
macros. The Rust destination preserves each exported macro as a public
constant, retains the signed C type of the un-suffixed shift and mask literals,
and retains the unsigned type and shift expressions of the encoded constants.
No configuration branch, function, data structure, or unsafe operation is
present in the source.

Direct UAPI consumers inspected: `include/uapi/linux/mman.h`,
`include/uapi/linux/shm.h`, and `include/uapi/linux/memfd.h`; each aliases the
shared macros without changing their values. No additional implementation
context is required by this header.
