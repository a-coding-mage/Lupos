# Implementation — S015124

Source: `vendor/linux/include/linux/sys.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The frozen x86_64 header closure selects this header only as an include of
`arch/x86/entry/syscall_32.c` and `arch/x86/entry/syscall_64.c`. Its include
guard is preprocessor-only. The nine legacy `_sys_*` aliases are all inside
`#ifdef notdef`; `notdef` is intentionally undefined, so this branch supplies
no active declarations or ABI surface. The destination therefore contains only
the required immutable provenance and a source-level explanation of the empty
active surface.

No ownership, locking, layout, linkage, or runtime behavior is introduced by
this header's active branch.
