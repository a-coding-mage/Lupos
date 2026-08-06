# Resolution — S013730, attempt 3

Reviewed the complete pinned `vendor/linux/include/linux/device-id/rpmsg.h`
and direct RPMSG consumers against the final candidate. This was source-only;
no compiler, formatter, linker, analyzer, test, or runtime command was used.

## Dispositions

### R1 — `RPMSG_DEVICE_MODALIAS_FMT` literal lowering

Resolved. Linux line 12 is an object-like macro expanding to the literal
`"rpmsg:%s"`. The candidate now uses the exported object-like Rust macro
`RPMSG_DEVICE_MODALIAS_FMT!()`, which expands to `b"rpmsg:%s\\0"`. The
expansion preserves all eight content bytes plus the terminating NUL and has
the fixed `[u8; 9]` array extent of the literal; a reference to that fixed
array is thin, and an FFI call obtains the C-style byte pointer explicitly with
`as_ptr()`. This replaces the rejected published slice reference.

The C literal-concatenation sites in `drivers/rpmsg/rpmsg_core.c` are
source-language token concatenation, not a runtime property of the header
data; their Rust translations must concatenate their own byte literals at their
respective task scopes. No additional RPMSG device-ID header state is omitted.

### R2 — `rpmsg_device_id.name` C `char` signedness

Resolved. The frozen x86_64 and AArch64 Phase 0 compile-command records pass
`-funsigned-char` with `-D__KERNEL__`; therefore the header's
`char name[RPMSG_NAME_SIZE]` is an inline 32-byte unsigned-character array in
both approved kernel contexts. `[u8; RPMSG_NAME_SIZE as usize]` preserves those
values and storage. `#[repr(C)]` keeps it immediately before the frozen
64-bit `unsigned long`/`u64` `driver_data` field, while `Clone, Copy` preserves
ordinary C aggregate-copy behavior.

### Upstream provenance correction

Resolved. The candidate's SPDX identifier now exactly matches the pinned
header: `GPL-2.0`, not `GPL-2.0-only`.

## Final task-local semantic closure

- `kernel_ulong_t` is active under the frozen `__KERNEL__` commands and is an
  unsigned 64-bit `long` on both selected targets, represented as `u64`.
- `RPMSG_NAME_SIZE` remains the C `int` literal `32`; its cast is limited to
  Rust's compile-time array-length syntax.
- No ownership, lifetime, locking, RCU, refcount, allocation, cleanup, or
  callable ABI behavior exists in this header beyond the C layout above.

Both reviewer reports are resolved by pinned-source and frozen-command
evidence. The source translation pipeline is complete only; it was not
compiled or tested.
