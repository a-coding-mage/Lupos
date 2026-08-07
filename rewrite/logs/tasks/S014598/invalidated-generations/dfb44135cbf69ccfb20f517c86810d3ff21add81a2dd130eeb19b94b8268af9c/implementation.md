# S014598 implementation

Task S014598 translates the complete pinned `vendor/linux/include/linux/pci_ids.h` header for x86_64 and AArch64. Every numeric `#define` is represented as a public `u32` constant, preserving the source identifier and literal value. Header guards and comments that do not define symbols are not emitted as Rust items. Source: Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

No compiler, formatter, test, or runtime command was run.
