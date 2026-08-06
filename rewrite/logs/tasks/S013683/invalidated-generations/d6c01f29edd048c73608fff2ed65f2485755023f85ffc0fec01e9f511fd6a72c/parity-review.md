# Parity review — S013683

Role: parity reviewer (slot 1)  
Model: gpt-5.6-terra  
Reasoning effort: high  
Scope: `include/linux/decompress/unxz.h` → `src/include/linux/decompress/unxz.rs`  
Result: ACCEPT — no source-parity findings.

## Source comparison

The candidate retains the source SPDX identifier and the required immutable provenance for the pinned Linux header, revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, common architecture membership, and task ID.

Linux declares one public interface, `int unxz(unsigned char *in, long in_size, long (*fill)(void *dest, unsigned long size), long (*flush)(void *src, unsigned long size), unsigned char *out, long *in_used, void (*error)(char *x))`. The candidate declares the same `unxz` linkage name as an unsafe C-ABI foreign function, with these exact ABI-level correspondences:

| Linux declaration | Candidate declaration |
| --- | --- |
| `int` result | `c_int` result |
| `unsigned char *in`, `unsigned char *out` | `*mut c_uchar` |
| `long in_size`, `long *in_used` | `c_long`, `*mut c_long` |
| `long (*)(void *, unsigned long)` callbacks | nullable `unsafe extern \"C\" fn(*mut c_void, c_ulong) -> c_long` callbacks |
| `void (*)(char *)` error callback | nullable `unsafe extern \"C\" fn(*mut c_char)` callback |

`Option<extern \"C\" fn>` preserves the C null-function-pointer representation for all three callbacks. The mutable pointee types preserve the header's non-const pointer contract. `c_long` and `c_ulong` preserve the target C `long` and `unsigned long` widths for both frozen x86_64 and AArch64 Linux targets.

The direct consumers are consistent with this interface: `lib/decompress.c` includes this header and places `unxz` in a `decompress_fn` slot; `lib/decompress_unxz.c` includes it in the non-`STATIC` build and defines the same parameter/result signature. Both frozen configurations select `CONFIG_DECOMPRESS_XZ=y` and `CONFIG_XZ_DEC=y`.

The C include guard has no runtime or exported-ABI effect after Rust module generation. No operative selected declaration, branch, callback contract, linkage name, or configuration-selected interface is omitted or changed.
