# S016277 implementation

Fresh Phase-1 implementation for `include/uapi/linux/netfilter/nf_tables.h`
into `src/include/uapi/linux/netfilter/nf_tables.rs`.

- Queue/lease: P02, attempt 1, `IN_PROGRESS`; immutable queue fingerprint
  `d6c01f29edd048c73608fff2ed65f2485755023f85ffc0fec01e9f511fd6a72c`.
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Architecture membership: common (the frozen x86_64 and AArch64 configurations
  select this UAPI header through recorded header-closure evidence).
- Source review: read the complete 2,022-line pinned UAPI header and the current
  Phase 0 task, ABI, symbol, identity, mapping, and frozen configuration facts.

The Rust file preserves every UAPI enum tag as a C-compatible integer alias and
every C global enumerator and object-like macro as a global Rust constant.
Implicit enum values preserve the C predecessor-plus-one rule; explicit values
and macro expressions retain their original operands and precedence. The one
high unsigned enum range is represented as `u32`; signed verdicts retain
`i32`; all other source enum tags retain the C target's ordinary signed
integer representation. There are no structs, unions, bitfields, functions, or
configuration-dependent layouts in the pinned header. The sole `__KERNEL__`
constant is included because this destination is the selected kernel-side UAPI
header under both frozen kernel configurations.

Source-only inventory check: all 962 source enum-tag/enumerator/object-macro
identifiers are present in the destination, with no extra translated ABI
identifiers. No compiler, formatter, test, build, analyzer, debugger, or
historical Lupos source was used.
