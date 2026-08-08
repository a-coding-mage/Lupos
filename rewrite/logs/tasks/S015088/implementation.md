# S015088 implementation

Fresh translation of `include/linux/sunrpc/gss_err.h` from pinned Linux commit
`425f94c2954b1fe80ebdbf9b29854e89750355df` for x86_64 and AArch64.

`OM_uint32` is represented exactly as `u32`. Every object-like macro is a
typed `u32` constant retaining its source value, including octal masks and the
`0xfffffffful` indefinite lifetime. The seven function-like source macros are
Rust declarative macros whose input expression occurs exactly once; they retain
the original masking, shifting, and field-extraction behavior. No configuration
conditional applies to this header under either frozen configuration.

No compiler, formatter, linker, test, or runtime command was run.
