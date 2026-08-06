# Resolution — S013683

Role: applier  
Model: gpt-5.6-terra  
Reasoning effort: high  
Scope: `include/linux/decompress/unxz.h` → `src/include/linux/decompress/unxz.rs`

I reopened the complete pinned header and the direct selected definition in
`lib/decompress_unxz.c`, together with the frozen x86_64 and AArch64 compile
commands and configurations. This was a source-only review; no compiler,
formatter, linker, test, or historical Lupos source was used.

## Finding dispositions

1. **Rust review: `error` pointee signedness. Accepted and fixed.**
   The pinned declaration is `void (*error)(char *x)`. The selected compile
   command for `lib/decompress_unxz.c` in both
   `rewrite/metadata/x86_64/compile_commands.json` and
   `rewrite/metadata/aarch64/compile_commands.json` includes `-funsigned-char`.
   Therefore the frozen C interface's `char *` element type is unsigned for
   both approved targets. The binding now uses
   `unsafe extern "C" fn(*mut c_uchar)` and no longer imports `c_char`.

2. **Rust review: nullable `error` callback. Accepted and fixed.**
   `lib/decompress_unxz.c:366-394` calls `error(...)` for every decoder-status
   failure and for each allocation-failure label without testing it for null.
   The binding now requires a non-null `unsafe extern "C" fn` for `error`;
   `fill` and `flush` remain `Option` because the implementation explicitly
   tests each against `NULL`. Its caller-contract documentation now states the
   required callback lifetime and failure-path behavior.

3. **Parity review: accept. Confirmed.**
   The corrected declaration still retains the exact exported spelling `unxz`,
   C ABI, result and scalar C widths, mutable input/output and `in_used`
   pointer contracts, and nullable `fill`/`flush` representation. The source
   header's `0BSD` SPDX identifier is preserved exactly, as are immutable
   provenance fields. The include guard has no exported or runtime effect in
   the Rust module.

## Final disposition

The selected header contains only this declaration. Its selected direct
definition and `lib/decompress.c` consumer establish the corrected callback
contract; no unresolved ownership, ABI, locking, refcount, RCU, or semantic
record remains for S013683. The task is ready for the queue `DONE` transition.
