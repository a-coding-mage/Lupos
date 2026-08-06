# S016353 implementation

Translated `include/uapi/linux/reboot.h` to
`src/include/uapi/linux/reboot.rs` from pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The destination retains every thirteen UAPI macro name and literal value.  Each
constant uses the type of its C unsuffixed integer literal: `u32` for values
outside the C `int` range and `i32` otherwise.  The pinned syscall declaration
uses `int` for magic values and `unsigned int` for commands; C's usual
arithmetic conversions therefore remain explicit at Rust call boundaries
rather than being hidden by changing the UAPI constant values.

No build, test, formatter, or compiler command was run.
