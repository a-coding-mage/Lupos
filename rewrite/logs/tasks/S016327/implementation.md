# Implementation evidence — S016327

- Task: `S016327`
- Pipeline/attempt: `P02` / `1`
- Linux source: `vendor/linux/include/uapi/linux/personality.h`
- Destination: `src/include/uapi/linux/personality.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected for both frozen configurations)

The complete pinned UAPI header was read. Its two anonymous C enumerations are
represented as explicitly `i32` public constants, matching the C enum constant
type. The `PER_CLEAR_ON_SETID` object-like macro is preserved as a public
constant expression over the same flag constants. All values and bitwise
combinations are transcribed directly from the pinned header; no Rust test,
compiler, formatter, or runtime command was used.

The C include guard has no additional runtime behavior in a path-preserving
Rust module, so it is represented by the module boundary rather than a second
definition mechanism.
