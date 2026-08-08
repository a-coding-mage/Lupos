# Parity review — S013677 / attempt 1

Reviewer: parity reviewer (independent source inspection)

Scope inspected:

- `vendor/linux/include/linux/decompress/generic.h`
- `src/include/linux/decompress/generic.rs` and its frozen `candidate.diff`
- direct selected uses in `vendor/linux/lib/decompress.c`, `vendor/linux/init/initramfs.c`, and `vendor/linux/init/do_mounts_rd.c`

Result: **APPROVE — no findings.**

The candidate's `decompress_fn` keeps every parameter and result from the C
typedef at generic.h:5: mutable byte buffers, LP64 `long` and `unsigned long`,
nullable fill/flush/error callbacks, nullable `posp`, and the integer result.
`Option<unsafe extern "C" fn>` preserves the nullable C function-pointer
positions used by `decompress_method` and its callers.  The declaration of
`decompress_method` keeps the const input buffer and mutable `const char **`
out-parameter (`*mut *const c_char`), including its permissible null value.

The direct consumers confirm those exact contracts: `initramfs.c:556-561`
obtains the nullable decompressor/name and invokes it with null stream
arguments as required; `do_mounts_rd.c:82-88` checks both output pointer and
function-pointer null states; `lib/decompress.c:63-83` conditionally writes
the nullable `name` out-parameter and returns either a selected callback or
null.  The Rust foreign declaration leaves that linkage and selection behavior
to the corresponding implementation translation; it neither invents a guard
nor changes any configuration-dependent behavior.

No compiler, formatter, analyzer, test, or runtime command was used.
