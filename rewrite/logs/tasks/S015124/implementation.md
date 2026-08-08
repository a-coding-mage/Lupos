# S015124 implementation

- Linux source: `include/linux/sys.h`
- Destination: `src/include/linux/sys.rs`
- Revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `x86_64`
- Queue lease: `P02`, attempt `1`

The complete pinned header contains only the `_LINUX_SYS_H` include guard,
comments, and an `#ifdef notdef` block of nine obsolete `_sys_*` aliases.
The frozen x86_64 configuration and original Kbuild command do not define
`notdef`; therefore the aliases are not emitted by the C preprocessor and
there are no selected declarations, function definitions, data objects, or
ABI symbols to translate. The x86 syscall dispatch sources include this
header but do not reference any alias from the inactive block.

The fresh Rust destination preserves the header's effective semantics as an
empty module, retaining only immutable provenance and an explanatory source
mapping. No facade constants, syscall wrappers, or invented linkage were
added.
