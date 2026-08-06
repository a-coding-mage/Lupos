# Implementation — S016454 (attempt 2)

- Oracle: `vendor/linux/include/uapi/linux/vesa.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Scope: common UAPI header, selected through both frozen configurations.
- Translated the `vesa_blank_mode` integer ABI and all four same-named C macro aliases.
- The C enum intentionally aliases `VESA_BLANK_MAX` to `VESA_POWERDOWN`; the Rust representation is a transparent `i32` wrapper with associated constants so the duplicate value remains representable.
