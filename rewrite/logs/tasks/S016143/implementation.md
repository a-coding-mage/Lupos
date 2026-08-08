# S016143 implementation

BLOCKED before source creation. The pinned `include/uapi/linux/hash_info.h`
contains the public tagged C declaration `enum hash_algo` and its implicitly
assigned enumerators. The frozen `rewrite/ABI.tsv` records the enum's ABI
representation as `PENDING_REVIEW` for both x86_64 and AArch64. A Rust enum
representation (including `#[repr(C)]`) cannot be selected without establishing
the required UAPI enum layout and FFI contract from the frozen evidence.

The exact enum ABI is therefore unresolved; creating a destination file would
guess at the contract. No source candidate was written, and no compiler,
formatter, linker, test, runtime, or historical Rust source was used.
