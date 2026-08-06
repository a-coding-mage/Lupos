# S016353 parity review (slot 1)

Reviewed `vendor/linux/include/uapi/linux/reboot.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/reboot.rs` and the S016353 scope/symbol records for
both frozen architectures.

## Result

No findings.

The candidate preserves all thirteen UAPI constant names and their exact
numeric values: the five `LINUX_REBOOT_MAGIC*` constants and the eight
`LINUX_REBOOT_CMD_*` constants.  Its explicit literal types match the C
unsuffixed-integer rules on both frozen 64-bit targets: `i32` for values
representable as `int` and `u32` for the hexadecimal values first represented
as `unsigned int`.  This includes `LINUX_REBOOT_MAGIC1` and the five command
values above `INT_MAX`; all remaining constants are `i32`.

The source path, exact pinned revision, `common` architecture membership, task
ID, and the original UAPI SPDX identifier are present and correct.  The C
include guard has no Rust source-level equivalent and introduces no exported
UAPI constant or runtime behavior to reproduce.

No source files were edited, and no build, test, formatter, or compiler command
was run.
