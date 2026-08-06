# Implementation — S016124

Translated `include/uapi/linux/falloc.h` to `src/include/uapi/linux/falloc.rs`.

The source contains nine UAPI fallocate flag macros.  Each is represented as a
public `i32` constant, preserving its C integer value and flag bit.  The source
has no types, functions, conditional configuration branches, or ABI layout.

No semantic question remains for this header: its macros are untyped C integer
constants and are used as fallocate mode bits, whose Linux interfaces use `int`.
