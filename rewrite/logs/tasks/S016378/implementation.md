# S016378 implementation

- Source: `vendor/linux/include/uapi/linux/serial_reg.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/serial_reg.rs`.
- Scope: common UAPI UART register offsets, bit values, composed expressions, and the `UART_FCR_R_TRIG_BITS(x)` parameterized expression for the frozen x86_64/aarch64 union.
- The 231 source definitions excluding the C include guard map one-for-one to 230 Rust constants and one `pub const fn`. C signed-int expressions use `i32`; the three OMAP base literals retain their C unsigned-int domain as `u32`.
- No conditional definitions occur in the pinned header. No tests, drivers, module indexes, or build actions were added.
