# S000772 implementation

Translated `arch/x86/include/uapi/asm/debugreg.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` into the path-preserving x86_64
UAPI Rust header.

The source contains only register-number and bit-encoding macros.  Every
selected macro is represented by a public constant with its C literal type:
the ordinary decimal/hex literals are `i32`, `DR6_RESERVED` is `u32` because
`0xFFFF0FF0` selects C `unsigned int`, and the selected non-`__i386__`
`DR_CONTROL_RESERVED` definition is `u64` because its C literal has the `UL`
suffix.  `DR_TRAP_BITS` retains its source OR expression and operand order.

The frozen task is x86_64 only.  Therefore the source's `__i386__` alternative
(`0xFC00`) is not emitted; its selected x86_64 `#else` value is preserved.
There is no storage, ownership, linkage, locking, allocation, or runtime
control flow in this header.

No compiler, formatter, linker, test, runtime, or historical translation was
used.
