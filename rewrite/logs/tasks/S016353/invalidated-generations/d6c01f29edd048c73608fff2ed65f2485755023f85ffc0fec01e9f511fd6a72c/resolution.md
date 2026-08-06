# Applier resolution — S016353

I reopened the complete pinned `vendor/linux/include/uapi/linux/reboot.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the fresh candidate, both frozen
architecture records, and both independent reports.

## Review dispositions

1. Parity review: accepted. The candidate declares every five
   `LINUX_REBOOT_MAGIC*` and eight `LINUX_REBOOT_CMD_*` names with the exact
   source bit pattern and has no omitted selected branch.
2. Rust review: accepted. This is a declarative constants-only UAPI header:
   it introduces no storage, layout, linkage, ownership, lifetime, locking,
   RCU, refcount, callback, allocation, cleanup, FFI, `unsafe`, or panic
   behavior.

## Final semantic mapping

The include guard at source lines 2--3 and 40 is an unconditional C
multiple-inclusion mechanism and has no Rust public-item, ABI, or runtime
counterpart. Each payload definition is an unconditional object-like macro.
On both frozen targets, the seven values representable as C `int` are public
`i32` constants: `LINUX_REBOOT_MAGIC2`, `LINUX_REBOOT_MAGIC2A`,
`LINUX_REBOOT_MAGIC2B`, `LINUX_REBOOT_MAGIC2C`,
`LINUX_REBOOT_CMD_RESTART`, `LINUX_REBOOT_CMD_CAD_OFF`, and
`LINUX_REBOOT_CMD_KEXEC`. `LINUX_REBOOT_MAGIC1` plus the five high command
literals first have C `unsigned int` type and are public `u32` constants. The
candidate retains every exact bit value; translated consumers must perform the
same explicit conversion at their operation boundary where C's usual
arithmetic conversions apply.

All 32 S016353 `SYMBOLS.tsv` rows are now `COMPLETE`, including the guard
handling and all thirteen value/type mappings for each frozen architecture.
The S016353 `SCOPE.tsv` semantic status is `COMPLETE`. There are no S016353
rows in `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, or `BLOCKERS.tsv`, which
is correct for this passive UAPI-header task. No task-local semantic question
remains pending.

All five required task evidence files exist. No compiler, formatter, linker,
test, runtime command, emulator, debugger, or benchmark was run. This marks
only source-translation-pipeline completion; it makes no build or test claim.
