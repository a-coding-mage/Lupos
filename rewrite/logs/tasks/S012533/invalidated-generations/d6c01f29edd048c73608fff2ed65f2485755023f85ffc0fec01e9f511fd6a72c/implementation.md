# S012533 implementation

Translated `include/asm-generic/device.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` into
`src/include/asm-generic/device.rs`.

The selected x86_64 and AArch64 header closure has the same complete source:
two intentionally memberless GNU C aggregate definitions, `struct dev_archdata`
and `struct pdev_archdata`.  The Rust candidate uses one zero-field
`#[repr(C)]` definition for each, preserving their role as by-value architecture
extension members.  The C include guard has no Rust item equivalent because the
destination is a single Rust module.

Context checked: `include/linux/device.h` embeds `dev_archdata` as
`device.archdata`; `include/linux/platform_device.h` embeds `pdev_archdata` as
`platform_device.archdata`.  No configuration conditional or exported function
is present in the pinned source.  No branding delta was made.
