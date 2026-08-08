# Rust semantics review — S016371, attempt 1, P02

Reviewed `vendor/linux/include/uapi/linux/seg6_genl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/seg6_genl.rs` and the sealed semantic proposal
`735801b74dec364aa5439246d44af696227ebdb39488e5459f90780afa2a97c7`.
This was a manual source review; no compiler, formatter, test, or
rust-analyzer diagnostic was invoked.

## Finding RUST-S016371-001 — C string-literal ABI is not preserved

**Severity:** high

`SEG6_GENL_NAME` is a C string literal in the UAPI header
(`seg6_genl.h:5`), therefore its character sequence includes the trailing NUL.
The direct pinned consumer initializes `struct genl_family.name` from it
(`vendor/linux/net/ipv6/seg6.c:494-497`); that field is a C character array
(`vendor/linux/include/net/genetlink.h:78-82`).

The candidate instead exposes `pub const SEG6_GENL_NAME: &str = "SEG6";`
at `seg6_genl.rs:7`. A Rust `&str` is a non-C-layout pointer/length pair and
its payload does not include the NUL terminator. It cannot stand in for the
C string-literal expression at FFI or when initializing the Linux-shaped
generic-netlink family name. The applier must provide a C-compatible,
NUL-terminated byte/static representation (and preserve any Rust-facing
view only as an additional non-ABI convenience) before closing this task.

Affected semantic records:

- `SC1-d9e89b57ec9ab851455ab7da16196e42004259912361c277d244bb1dfbf19b9d`
- `SC1-741a39a13b2e5845da0fd48f2ce5f97c0cab53e5e4a8bf18624cc75a164e19e1`
- `SC1-a6f8069867d1f4ae8994c4c51963a1c57f6ae926351acb52e8a36f94ad95a9db`
- `SC1-b45752b16f7cc008f0c90cb7001ae4c88f7fdef7322911d9c81fcf0d45bd52b0`

## Checked without further findings

Both anonymous C enums declare no named enum object or ABI-passed enum type;
their enumerators are C `int` constants. The candidate's `i32` values,
including both `*_MAX` subtractions, preserve every selected value and the
signed constant domain for x86_64 and AArch64. `SEG6_GENL_VERSION` is likewise
the C `int` constant `1`; its later conversion to `struct genl_family.version`
(`unsigned int`) is exact. This header contains no pointers, ownership,
borrowing, interior mutability, callbacks, allocation, `Drop`, `unsafe`, or
panic-capable operations. No `repr(C)` type is required for the anonymous
enumerations because this header exports constants only, not a representable
enum object.

**Result:** FINDINGS — do not accept the candidate unchanged.
