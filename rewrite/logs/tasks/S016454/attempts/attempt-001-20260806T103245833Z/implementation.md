# Implementation — S016454

Translated `include/uapi/linux/vesa.h` to `src/include/uapi/linux/vesa.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header is unconditional for the frozen common x86_64/aarch64 configuration users.  It defines one C-ABI `enum vesa_blank_mode`: the three explicit levels retain their C `int` values 0, 1, and 2; `VESA_POWERDOWN` remains the bitwise-OR expression of the VSYNC and HSYNC levels; and `VESA_BLANK_MAX` remains equal to `VESA_POWERDOWN`.  The four self-referential C preprocessor macro aliases are represented as public `c_int` constants with the same names and values.  No structures, functions, storage, configuration branches, ownership, locking, error, or cleanup paths occur in this header.

The source license, immutable provenance, and `common` architecture category match the task row.  No branding delta applies.
