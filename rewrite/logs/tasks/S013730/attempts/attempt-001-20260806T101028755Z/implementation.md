# S013730 implementation

Translated `include/linux/device-id/rpmsg.h` to the leased destination only.

- `kernel_ulong_t` is `unsigned long`; both frozen targets set `CONFIG_64BIT=y`, so the mapping is `u64`.
- The frozen compile commands use `-funsigned-char`; the 32-byte `name` field is therefore `[u8; RPMSG_NAME_SIZE]`.
- `#[repr(C)]` preserves the C field order and alignment: 32-byte name followed by an 8-byte unsigned long, for a 40-byte record on both targets.
- The modalias macro is represented as its exact NUL-terminated C string-literal bytes, `b"rpmsg:%s\\0"`.

Source context checked: `drivers/rpmsg/rpmsg_core.c` compares/copies the fixed name array and assigns `driver_data`; the selected AArch64 `net/qrtr/smd.c` table uses aggregate initialization. No compiler, formatter, test, or historical Rust source was used.
