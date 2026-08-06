# S016378 applier resolution

Reopened the complete pinned `vendor/linux/include/uapi/linux/serial_reg.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate, both independent
reviews, and the pinned consumer in
`vendor/linux/drivers/tty/serial/8250/8250_port.c:2978`.  That consumer passes
`up->fcr`, declared `unsigned char` in
`vendor/linux/include/linux/serial_8250.h:132`.

## P1 / R1 — resolved

Upstream `UART_FCR_R_TRIG_BITS(x)` (serial_reg.h:101-102) evaluates `x` once,
masks it with an `int` constant, and right shifts the result.  On both frozen
architectures, `int` is 32 bits.  Thus `unsigned char`, signed/unsigned
16-bit, and signed-32-bit inputs are promoted to `int`; `unsigned int` keeps
its unsigned 32-bit result; and 64-bit signed/unsigned integer inputs retain
their respective 64-bit arithmetic-conversion result types.

The fixed `i32` function was replaced with a public input-mapping trait and a
single-evaluation generic function.  The mapping explicitly implements those
frozen C conversion classes: `u8` (the pinned `up->fcr` caller) produces the
promoted `i32` expression result without a caller cast; unsigned 32-bit
inputs remain `u32`; and signed/unsigned 64-bit and pointer-width integer
inputs retain their corresponding result types.  No fixed `i32` argument
substitution remains.

The header has no ownership, lifetime, locking, layout, linkage, or
configuration semantics beyond its include guard.  Its two
`UART_FCR_R_TRIG_BITS` inventory entries (x86_64 and aarch64) are therefore
closed by this explicit mapping; all other definitions remain the reviewed
literal or composed constants.  No branding difference was introduced.

No compiler, formatter, build, test, or runtime command was run.
