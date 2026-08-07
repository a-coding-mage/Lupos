# S016378 implementation

- Source: `vendor/linux/include/uapi/linux/serial_reg.h`
- Revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/uapi/linux/serial_reg.rs`
- Architectures: `common`
- Lease: `P02`, attempt 2

Translated every selected register offset, bit mask, alias, shift, arithmetic
constant, and the `UART_FCR_R_TRIG_BITS(x)` macro as a Rust macro, preserving
the source order and integer widths. Negative RSA offsets remain signed; OMAP
physical base constants are unsigned 32-bit values. No source-level uncertainty
was encountered.
