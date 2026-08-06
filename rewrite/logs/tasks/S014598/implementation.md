# S014598 implementation record

Pinned source: `vendor/linux/include/linux/pci_ids.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The common x86_64/AArch64 header defines 2,902 numeric PCI class, vendor,
subvendor, device, and subdevice identifiers. Each C integer literal macro was
translated to a public Rust `i32` constant with the same name and literal
value. The C include guard has no Rust runtime equivalent and is omitted.

This is a fresh attempt-2 source translation. No historical Lupos source,
prior S014598 evidence, compiler, formatter, or test output was used.
