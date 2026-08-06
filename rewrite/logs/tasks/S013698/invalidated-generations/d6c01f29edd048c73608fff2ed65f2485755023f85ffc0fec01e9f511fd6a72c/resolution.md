# S013698 applier resolution

Resolved against the complete pinned
`vendor/linux/include/linux/device-id/auxiliary.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64/AArch64
configurations, scope/task/symbol/ABI/lifetime records, and the relevant
upstream consumers in `drivers/base/auxiliary.c` and
`scripts/mod/file2alias.c`. The task remains the frozen `common` mapping to
`src/include/linux/device-id/auxiliary.rs`.

## Finding dispositions

1. **Parity P1 / Rust required — `name` character signedness:** accepted and
   fixed. The frozen compile-command evidence records `-funsigned-char` for
   both selected architectures, so upstream line 13's `char[40]` has unsigned
   byte values. `auxiliary_device_id::name` is now `[u8;
   AUXILIARY_NAME_SIZE as usize]`; `#[repr(C)]` retains its consecutive
   40-byte C layout.

2. **Parity P1 / Rust required — module-prefix C literal:** accepted and
   fixed. Upstream line 10 supplies the bytes `auxiliary:` plus its implicit
   trailing NUL; `drivers/base/auxiliary.c:206` consumes that literal as a
   `%s` argument and `scripts/mod/file2alias.c:1349` uses it in adjacent
   literal concatenation. The Rust item is now immutable static byte storage
   `[u8; 11] = *b"auxiliary:\\0"`, rather than a fat, non-NUL `&str`.
   Rust call sites needing a C pointer must use the byte storage's pointer,
   and a translated equivalent of the C token-concatenation use must compose
   byte literals explicitly; the original script remains its pinned C source.

3. **Parity P2 / Rust required — `AUXILIARY_NAME_SIZE` integer type:**
   accepted and fixed. The public constant is now `core::ffi::c_int = 40`,
   matching the frozen C unsuffixed integer literal's signed 32-bit `int`
   representation. The one Rust-only array-bound conversion is explicit
   (`as usize`) and does not change the exported macro representation.

4. **Rust required — ordinary C aggregate copying:** accepted and fixed.
   The upstream aggregate contains only the 40-byte character array and
   `kernel_ulong_t`, so C permits value copies. The Rust representation now
   derives `Copy, Clone`, preserving bytewise value-copy use without changing
   layout or adding ownership/destruction behavior.

## Semantic-record closure

- The selected `__KERNEL__` branch is active in both frozen command contexts;
  `kernel_ulong_t` is therefore present and is represented by
  `core::ffi::c_ulong`, the LP64 `unsigned long` on x86_64 and AArch64.
- `auxiliary_device_id` has no packing, alignment, pointer-ownership,
  refcount, locking, RCU, allocation, or cleanup contract in the pinned
  header. `#[repr(C)]`, field order, an unsigned 40-byte array, and `c_ulong`
  preserve its ABI-relevant layout; `Copy` adds no drop behavior.
- The header guard is preprocessing-only and has no separate Rust ABI item.
  No branding delta was introduced.

No compiler, formatter, rust-analyzer, linker, build, test, debugger, or
runtime tool was invoked during application.
