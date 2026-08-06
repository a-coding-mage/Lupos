# Implementation — S013727

Translated `include/linux/device-id/platform.h` to
`src/include/linux/device-id/platform.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected common x86_64/aarch64 kernel branch contains the
`kernel_ulong_t` alias (`unsigned long`), the two object-like platform ID
macros, and C-layout `struct platform_device_id`. The Rust alias preserves the
frozen LP64 C type; the name field remains a fixed-width unsigned C `char`
array because both frozen Kbuild commands select `-funsigned-char`, and the
driver data field remains an unsigned machine word. The macro translations
retain C `int` literal type for the array bound and a NUL-terminated expansion
for the module-prefix string.

Phase 0 scope, mapping, symbols, ABI/lifetime records, both frozen
configurations, header closure, and consuming platform/mod-devicetable context
were inspected. The header has no functions, storage, ownership, locking,
error, or cleanup paths. No branding delta applies.

No compiler, formatter, test, runtime command, historical Rust source, or
non-leased source file was used or changed.
