# Implementation — S016241

Translated `vendor/linux/include/uapi/linux/membarrier.h` to
`src/include/uapi/linux/membarrier.rs` for the common x86_64/AArch64 scope.

The source contains two C enums and no configuration-selected branches beyond
the C header guard. Both tags are represented as C-`int` ABI type aliases; all
enumerators retain their source names and integral values, including the
`MEMBARRIER_CMD_SHARED` compatibility alias.

No ownership, allocation, locking, or executable control-flow behavior exists
in this UAPI declaration-only header.
