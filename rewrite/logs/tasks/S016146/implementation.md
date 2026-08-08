# S016146 implementation

- Linux source: `vendor/linux/include/uapi/linux/hid.h`
- Destination: `src/include/uapi/linux/hid.rs`
- Revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`
- Selected inventory: `_UAPI__HID_H`; USB interface class/subclass/protocol constants; `hid_report_type` and its five values; `hid_class_request` and its six values; `HID_DT_HID`, `HID_DT_REPORT`, `HID_DT_PHYSICAL`, and `HID_MAX_DESCRIPTOR_SIZE`.

The fresh path-preserving Rust file retains the upstream SPDX and copyright
notices, C-compatible enum representations, exact enum values, and integer
macro values. The descriptor constants preserve the pinned `USB_TYPE_CLASS`
expansion from `include/uapi/linux/usb/ch9.h` (`0x01 << 5`) because the selected
header has no Rust module import for that unselected dependency.

No compiler, formatter, linker, test, runtime, or historical Lupos source was
used.
