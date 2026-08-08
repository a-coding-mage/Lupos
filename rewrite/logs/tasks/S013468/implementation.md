# S013468 implementation

- Task: `S013468`
- Pipeline/attempt: `P01` / `1`
- Linux source: `vendor/linux/include/linux/asn1.h`
- Destination: `src/include/linux/asn1.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (the header is selected for both frozen configurations)

The complete pinned header contains only the `_LINUX_ASN1_H` include guard, the
`asn1_class`, `asn1_method`, and `asn1_tag` C enums, and the
`ASN1_CLASS_BITS`, `ASN1_CONS_BIT`, and `ASN1_INDEFINITE_LENGTH` integer
macros. The Rust translation preserves each enumerator's numeric value,
including the reserved tag values 14 and 15 remaining absent, and uses
`#[repr(C)]` for the three C-visible enum types. Enumerator names are re-exported
at module scope to preserve the C header's unqualified names. Integer macros
remain `i32` constants, matching C integer-literal semantics.

There are no functions, statics, pointer-bearing declarations, conditional
configuration branches, or cleanup paths in the pinned header beyond its
include guard. No tests, drivers, compiler commands, or historical Lupos source
were used.
