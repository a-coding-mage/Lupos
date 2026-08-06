# S013661 parity review (slot 1)

Reviewer: parity reviewer (`gpt-5.6-terra`, high)  
Scope: `include/linux/crc32poly.h` → `src/include/linux/crc32poly.rs`  
Pinned revision verified: `425f94c2954b1fe80ebdbf9b29854e89750355df` (`vendor/linux.SHA` and `vendor/linux` HEAD)  
Queue verification: task `S013661` is `REVIEWING`, pipeline `P01`, destination and Linux paths match the frozen row, and the frozen architecture set is `common`.

## Source evidence examined

- Complete pinned header: `vendor/linux/include/linux/crc32poly.h`.
- Frozen scope/symbol/header-closure records for `S013661`.  The header is selected for both approved architectures through built-in `lib/decompress_bunzip2.o`; its sole selected Rust consumer is `lib/decompress_bunzip2.c` / `S017216`.
- The complete relevant CRC initialization context in `vendor/linux/lib/decompress_bunzip2.c` (`start_bunzip`): `c` is `unsigned int` and uses `c = c & 0x80000000 ? (c << 1) ^ CRC32_POLY_BE : (c << 1)`.
- Non-selected original users: the AArch64 `xgbe-dev.c` driver also XORs `CRC32_POLY_LE` into a `u32`; the out-of-scope generator uses all three values. These corroborate the literal values/types but do not expand the task scope.

## Finding P1 — `CRC32_POLY_BE` is not a context-equivalent replacement for the C macro

**Severity: major.**  Linux line 9 defines the untyped macro expansion `0x04c11db7`. On both frozen targets that hexadecimal unsuffixed literal has type `int`; in the selected `start_bunzip` expression, the usual arithmetic conversions convert it to `unsigned int` before the XOR because the other operand is `unsigned int`. The resulting 32-bit bit pattern is `0x04c11db7`.

The candidate fixes the exported item’s type as `pub const CRC32_POLY_BE: i32`. It accurately describes the literal’s standalone C type, but Rust performs no corresponding usual arithmetic conversion: a Rust `u32 ^ CRC32_POLY_BE` (and a `u32` conditional arm paired with it) is type-incompatible rather than converting the `i32` value to `u32`. Therefore the candidate does not preserve the selected caller’s macro-expansion semantics by itself; a later caller-side cast would be a separate, easy-to-omit semantic dependency and is not represented by this header translation.

Resolution required: represent the selected expression behavior with an explicit, reviewed `u32` value/conversion at the Rust interface used by the translated decompressor, while retaining a documented mapping for the source macro’s standalone `int` literal semantics. Reopen the selected caller and header mapping together; do not rely on the candidate comment as an implementation of C promotions.

## Verified items without finding

- Exact values: `CRC32_POLY_LE = 0xedb88320`, `CRC32_POLY_BE = 0x04c11db7`, and `CRC32C_POLY_LE = 0x82f63b78` match their source literals. The first and third correctly require `unsigned int` / `u32` at 32 bits; the second fits `int` / `i32` as a standalone literal.
- All three C macro replacements are side-effect-free constant expressions; no evaluation-order or repeated-evaluation behavior is lost for their current uses.
- Public Rust identifiers preserve the three macro names exactly. No branding delta is present.
- The immutable provenance header gives the correct Linux path, exact pinned revision, `common` architecture set, and task ID. Its mandated SPDX form is present.
- The C include guard has no independent runtime or expression behavior to reproduce in the path-preserving Rust module.

No compiler, formatter, rust-analyzer diagnostic, build, test, debugger, or runtime command was used. No source or queue file was edited.
