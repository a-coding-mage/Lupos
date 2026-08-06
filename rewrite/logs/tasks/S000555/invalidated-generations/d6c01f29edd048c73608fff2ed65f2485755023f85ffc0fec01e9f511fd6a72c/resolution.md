# Resolution — S000555

Applier: `applier` (`gpt-5.6-terra`, high)  
Task: `arch/x86/include/asm/inat_types.h` ->
`src/arch/x86/include/asm/inat_types.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`  
Configuration: frozen x86_64 (`CONFIG_X86=y`, `CONFIG_X86_64=y`, 64-bit
`x86_64-linux-gnu` Kbuild invocation recorded in `rewrite/FILE_MAP.tsv`).

This was a manual, source-only application. No compiler, formatter, analyzer,
build, test, debugger, or runtime tool was run.

## Review dispositions

1. **Parity P1 / Rust R1 — accepted and fixed.**  Pinned
   `vendor/linux/arch/x86/include/asm/inat_types.h:1` is
   `GPL-2.0-or-later`.  The destination SPDX line is corrected to that exact
   identifier.  This retains the upstream license identifier rather than
   narrowing it.

2. **Parity P2 — resolved by explicit source-backed disposition; no Rust
   replacement is added.**  Lines 2–3 of the pinned header define
   `_ASM_X86_INAT_TYPES_H`, and line 15 closes the `#ifndef`; the selected
   `ifdef@2`, `endif@15`, and `operative_macro` records in
   `rewrite/SYMBOLS.tsv` are therefore accounted for.  A complete pinned-tree
   search for the exact token finds only this header and the separate
   `tools/arch/x86/include/asm/inat_types.h` copy; no selected source tests,
   consumes, or branches on the macro.  Its whole effect in this header is
   C-preprocessor multiple-inclusion suppression.  The selected Rust source
   has no C preprocessor and does not provide a C header to C consumers; Rust
   module inclusion is controlled by the later deterministic module-index
   generation, which is outside this file task.  Thus the macro and its paired
   conditional have no Rust runtime, type, layout, linkage, or cross-language
   ABI contract to translate.  A Rust `macro_rules!`, constant, or exported
   item would introduce a new contract and is not a faithful mapping.

## Final semantic closure

- `insn_attr_t` (line 11): `unsigned int` is a 32-bit unsigned scalar under
  the frozen x86_64 ABI; mapped to public alias `u32`.
- `insn_byte_t` (line 12): `unsigned char` is an 8-bit unsigned scalar;
  mapped to public alias `u8`.
- `insn_value_t` (line 13): `signed int` is a 32-bit signed scalar; mapped to
  public alias `i32`.

The direct selected context confirms the aliases' required use: `inat.h`
declares the instruction-attribute APIs using `insn_attr_t` and `insn_byte_t`,
while `insn.h:16-24` overlays `insn_value_t` with `insn_byte_t bytes[4]`.
The aliases preserve exactly these widths, signedness, alignment, and by-value
representation. They introduce no storage, ownership, lifetime, locking,
RCU, refcount, cleanup, linkage, or `unsafe` boundary.

Accordingly, every S000555 `PENDING_REVIEW` record is resolved as follows:

- `ifndef@2`, `_ASM_X86_INAT_TYPES_H`, and `endif@15`: C-preprocessor-only
  include-guard contract; **NOT_APPLICABLE to Rust source semantics**, with no
  selected consumer of the macro.
- all three aliases: **closed** with the scalar mappings above; no additional
  ABI or lifetime contract exists in the source.

No unresolved source, scope, ABI, lifetime, ownership, locking, or
cross-language contract remains for this task.
