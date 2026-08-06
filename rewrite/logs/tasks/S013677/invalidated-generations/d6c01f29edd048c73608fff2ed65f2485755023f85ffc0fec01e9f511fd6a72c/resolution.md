# Resolution — S013677

Reviewed the complete pinned `include/linux/decompress/generic.h` and its
`decompress_method` implementation in `lib/decompress.c:63-87` at Linux
revision `425f94c2954b1fe80ebdbf9b29854e89750355df`. No compiler, formatter,
build, test, runtime, or benchmark command was run.

## R1 — accepted

The parity review found the `decompress_fn` declaration faithful. The final
alias remains a nullable `unsafe extern "C"` function pointer, with each
nullable callback represented independently and all seven parameters in the
source order. The external `decompress_method` declaration remains an FFI
declaration with its source mutability, `long` widths, and nullable result.

## R2 — accepted and corrected

The Rust review correctly found that the candidate documentation imposed a
stronger input-buffer condition than the pinned implementation. The final
declaration now says exactly that `len < 2` causes no `inbuf` read and returns
null (storing null through a non-null `name`), while `len >= 2` requires only
two readable input bytes. It also records the unknown-magic case, where the
sentinel produces null through `name` and a null decompressor. No executable
FFI type or calling convention changed.

The S013677 symbol, ABI, and lifetime records are complete for both frozen
architectures: the C include guard has no Rust runtime mapping, the nullable
C callback ABI is retained, and the declaration takes neither ownership nor a
Rust reference to any Linux-controlled buffer or callback.
