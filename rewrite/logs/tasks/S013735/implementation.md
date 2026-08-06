# Implementation — S013735

Translated `include/linux/device-id/spi.h` to
`src/include/linux/device-id/spi.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

- Both frozen kernel configurations select the `__KERNEL__` typedef. Their
  x86_64 and AArch64 targets are 64-bit LP64, so C `unsigned long` is retained
  as `u64` for `kernel_ulong_t`.
- `SPI_NAME_SIZE` retains its C `int` literal value and `SPI_MODULE_PREFIX`
  remains a macro expansion of the NUL-terminated C string literal rather than
  an addressable Rust static.
- `spi_device_id` is `#[repr(C)]`; its `char[32]` field is `[u8; 32]` because
  both frozen command lines specify `-funsigned-char`, followed by the
  machine-word `driver_data` field.

No compiler, formatter, linker, test, or runtime command was run.
