# Implementation — S016053

Translated `include/uapi/linux/arm_sdei.h` into the path-preserving
`src/include/uapi/linux/arm_sdei.rs` for the frozen aarch64 configuration.

The source contains all 51 operative UAPI macros recorded in `SYMBOLS.tsv`:
the SDEI v1.0 SMCCC function-number family, version extraction family, return
values, registration/status/completion values, and GET_INFO selectors/results.
There are no C structs, enums, typedefs, or externally linked declarations in
the pinned header.

The two hexadecimal base/mask literals are C `unsigned int` values on the
frozen AArch64 ABI and are represented as `u32`. `SDEI_1_0_FN` uses
`wrapping_add`, preserving unsigned-C arithmetic rather than Rust debug-mode
overflow behavior. Version extraction works on the 64-bit firmware version
word (`u64`) and returns its masked field as `u64`. The remaining unsuffixed
decimal literals have C `int` type and are represented as `i32`.

No build, formatter, compiler, test, or runtime command was run.
