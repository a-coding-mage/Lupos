# S013730 implementation — attempt 2

Implemented `include/linux/device-id/rpmsg.h` as
`src/include/linux/device-id/rpmsg.rs` from the pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

- `kernel_ulong_t` remains C `unsigned long` through `core::ffi::c_ulong`.
  Both frozen kernel targets are 64-bit, preserving the field's ABI width and
  alignment.
- `RPMSG_NAME_SIZE` remains the source's C integer literal `32`; its explicit
  `c_int` representation is converted only where Rust requires an array length.
- `RPMSG_DEVICE_MODALIAS_FMT` retains the exact C literal bytes and terminating
  NUL (`b"rpmsg:%s\\0"`).
- `rpmsg_device_id` is `#[repr(C)]`, uses an inline `[u8; 32]` name field
  because the frozen C commands select unsigned `char`, and derives `Copy` so
  value copies retain C struct-copy behavior. `driver_data` remains after the
  name field as in the source.

Source context checked: the complete source header, `include/linux/rpmsg.h`,
`drivers/rpmsg/rpmsg_core.c`, selected rpmsg ID-table consumers, frozen
configuration rows, and Phase 0 metadata. No compiler, formatter, test, or
historical translation source was used.

Implementation model: Terra, medium reasoning effort.
