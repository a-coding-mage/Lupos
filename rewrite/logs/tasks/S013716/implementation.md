# S013716 implementation

Translated `include/linux/device-id/isapnp.h` from the pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` into the leased destination.

- Preserved `kernel_ulong_t` as C `unsigned long` via `core::ffi::c_ulong` for
  both frozen 64-bit targets.
- Preserved `ISAPNP_ANY_ID` as the C unsuffixed integer literal `0xffff`.
- Preserved the `isapnp_device_id` field order and C layout with `#[repr(C)]`:
  four `unsigned short` identifiers followed by `driver_data`.

No source behavior was inferred beyond the complete pinned header and its
frozen common-target manifest records.
