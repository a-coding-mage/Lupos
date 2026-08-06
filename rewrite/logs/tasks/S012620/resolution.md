# S012620 resolution

Applied after reopening pinned `vendor/linux/include/crypto/dh.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` and the concrete helper behavior
in `vendor/linux/crypto/dh_helper.c`. No build, formatter, test, or runtime
command was run.

## P1 — resolved

The destination provenance SPDX expression now exactly matches upstream:
`GPL-2.0-or-later`. No branding or other source identity was changed.

## RUST-001 — resolved

The declarations now use a Rust-2024 `unsafe extern "C"` block. The four
functions remain raw-pointer C ABI declarations, so calls retain the upstream
caller obligations for pointer validity, accessible ranges, mutability, and
the decode buffer's aliasing lifetime. No function was made a safe FFI call
surface.

## Recheck

`struct dh` remains `#[repr(C)]` with its three `const void *` fields and
three `unsigned int` fields in exact C declaration order. All four function
names, signatures, pointer mutability, aliases, and C ABI remain unchanged.
