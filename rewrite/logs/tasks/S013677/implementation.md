# Implementation evidence

- Task: `S013677`
- Pipeline/attempt: `P01` / `1`
- Linux source: `vendor/linux/include/linux/decompress/generic.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/linux/decompress/generic.rs`
- Architectures: `common` (the frozen x86_64 and AArch64 records agree)

The complete pinned header was read. Its implementation-bearing surface is the
`decompress_fn` callback typedef and the `decompress_method` declaration; the
include guard and comments have no Rust runtime representation. The callback is
represented as an optional unsafe C-ABI function pointer because the Linux
typedef is a nullable function pointer and the decompressor table uses NULL
entries. Its callback parameters preserve the C pointer mutability, `long`,
`unsigned long`, and `char *` types using `core::ffi` widths. The exported
declaration preserves the const input buffer, writable `const char **` output
slot, C ABI, and callback return type. The Linux `__init` annotation is a
linker/section attribute and has no declaration-level Rust mapping in the
frozen records, so no unsupported section claim was added.

Relevant pinned consumers were read: `vendor/linux/lib/decompress.c`,
`vendor/linux/init/do_mounts_rd.c`, and `vendor/linux/init/initramfs.c`. They
confirm nullable decompressor values and the callback signature. No compiler,
formatter, build, test, or runtime command was run.
