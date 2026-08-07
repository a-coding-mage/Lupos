# S016105 implementation

- Linux source: `vendor/linux/include/uapi/linux/dpll.h`
- Revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/uapi/linux/dpll.rs`
- Architectures: `x86_64,aarch64` (common)
- Scope: all declarations selected for S016105: family/version constants,
  every DPLL enum and enumerator (including private/max sentinels), frequency
  and divider constants, and multicast-group name.

The fresh Rust file uses `#[repr(i32)]` for each C enum, preserving Linux's
C `int` underlying representation and discriminant ordering. Integer macros
are explicitly `i32`; string macros are `&str`. Max aliases retain the exact
`(__MAX - 1)` values from the header. No conditional branches or generated
declarations were omitted; no implementation-specific behavior was inferred.
