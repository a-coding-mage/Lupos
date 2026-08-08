# Rust review — S013677 (attempt 1, slot 2)

Reviewed independently from the candidate, its snapshot, the pinned Linux
header, and the direct pinned consumers in `lib/decompress.c`,
`init/initramfs.c`, and `init/do_mounts_rd.c`.  No compiler, formatter,
analyzer, test, or historical Lupos source was used.

## Result: APPROVE

`decompress_fn` retains the C function-pointer ABI: `c_int`, `c_long`, and
`c_ulong` preserve the selected scalar contracts; the nested callbacks use the
C calling convention; and `Option<unsafe extern "C" fn(...)>` represents the
nullable function-pointer cases used by the decompressor table and callers.
The `*mut c_void`, mutable byte, mutable position, and mutable error-message
pointers deliberately retain C's unbounded aliasing, nullability, and
caller-controlled lifetime rather than manufacturing Rust references.

`decompress_method` retains a const input buffer and the writable
`const char **` output slot as `*const u8` and `*mut *const c_char`.
The direct implementation writes that slot only when non-null and returns a
nullable decompressor function pointer, matching the candidate declaration.
No ownership transfer, allocation, `Drop`, pinning, interior mutability,
`Send`/`Sync` assertion, or callback lifetime extension is introduced.

This header has no layout-bearing aggregate, no unsafe block, and no Rust
reference or bounds operation.  The candidate adds no guard replacement,
module-state mechanism, exported-Rust symbol, panic path, or test-only path;
the C header guard has no runtime ABI counterpart.
