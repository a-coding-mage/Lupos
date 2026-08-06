# Parity review — S013730, attempt 3, slot 1

Reviewed only `vendor/linux/include/linux/device-id/rpmsg.h` against
`src/include/linux/device-id/rpmsg.rs`, with the selected task manifests,
frozen configurations, and direct `include/linux/rpmsg.h` context.

## Result

Accepted: no parity findings.

## Literal source comparison

- Both selected configurations assert `CONFIG_64BIT=y`; the original
  `kernel_ulong_t` is therefore the 64-bit `unsigned long` in the frozen
  kernel compilation contexts (`__KERNEL__` is present). `u64` preserves that
  width and unsigned representation for both x86_64 and aarch64.
- `RPMSG_NAME_SIZE` retains the original unsuffixed C `int` literal value 32;
  the Rust array uses the corresponding `usize` conversion solely for its
  array-length position.
- `RPMSG_DEVICE_MODALIAS_FMT` retains all eight literal bytes and the trailing
  NUL of C `"rpmsg:%s"`; its `&[u8; 9]` representation preserves the fixed
  literal extent and a thin reference to it.
- `#[repr(C)] rpmsg_device_id` preserves declaration order: a 32-byte `name`
  array followed at offset 32 by the 8-byte `driver_data`, for a 40-byte,
  8-byte-aligned record on both frozen targets. `Clone, Copy` preserves the C
  record's ordinary by-value copy semantics without changing its layout.
- The candidate contains every selected macro/type/conditional-bearing source
  element, no unallowlisted branding, and immutable provenance matching
  `vendor/linux.SHA` (`425f94c2954b1fe80ebdbf9b29854e89750355df`).

No compiler, formatter, linker, test, or runtime command was used.
