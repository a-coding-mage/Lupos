# Implementation — S016242

Source oracle: `vendor/linux/include/uapi/linux/memfd.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected source is an unconditional common UAPI header.  It defines five
`unsigned int` `memfd_create(2)` flags, then aliases the common huge-page
encoding shift, mask, and eleven size values from
`include/uapi/asm-generic/hugetlb_encode.h` (task `S016005`, DONE).  The Rust
translation preserves all twenty-one selected `MFD_*` names: unsigned C
constants are `u32`; the included shift and mask remain `i32`, matching their
un-suffixed C integer literals.  No configuration branches apply beyond the
header guard, and no ABI objects, ownership, locking, or lifetime behavior are
introduced by this constants-only header.

Inspected direct consumers include `mm/memfd.c`, which combines the flags,
validates the encoded huge-page field, and extracts it with the exported shift
and mask.  No source was compiled, formatted, or executed.
