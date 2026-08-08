# S014172 implementation

- Task: `S014172`
- Pipeline/attempt: `P02` / `1`
- Linux source: `include/linux/kern_levels.h`
- Destination: `src/include/linux/kern_levels.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by both frozen configurations)
- Source review: read the complete pinned header and its direct printk/loglevel consumers.

The fresh translation preserves the include guard's effect through a
path-local Rust module boundary, retains the KERN_* definitions as
token-producing declarative macros so severity prefixes remain string literals
for format-string concatenation, preserves the SOH character and ASCII value,
and maps the integer LOGLEVEL_* values to fixed-width signed constants.
No tests, compiler, formatter, or runtime tooling was run.

Destination SHA-256: `63c8937426de71e8af1a8cf6e26c19cb3f48dbf8cd2c34e59721605b49281372`
