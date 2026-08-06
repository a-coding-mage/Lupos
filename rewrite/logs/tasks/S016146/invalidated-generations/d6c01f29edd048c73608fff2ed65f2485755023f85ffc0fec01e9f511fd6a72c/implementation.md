# Implementation — S016146

Translated `include/uapi/linux/hid.h` to `src/include/uapi/linux/hid.rs` from
the pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The candidate retains every selected HID USB class/subclass/protocol constant,
both C enum tags and their zero-based enumerators, all HID class requests, the
three class descriptor constants, and the descriptor-size limit.  The two
named C enum tags are represented by `c_int` aliases, matching the selected
targets' C enum representation.  Descriptor constants retain the chapter-9
`USB_TYPE_CLASS` expression (`0x01 << 5`) because the upstream header obtains
that macro from its including context rather than declaring it itself.

No structs, functions, allocation, ownership transfer, locking, or runtime
behavior are present in this UAPI header.  No build, formatter, test, or other
runtime command was run.
