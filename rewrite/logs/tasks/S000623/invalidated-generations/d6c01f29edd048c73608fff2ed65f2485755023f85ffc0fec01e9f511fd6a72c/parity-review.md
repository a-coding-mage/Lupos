# Parity review — S000623

## Outcome

No source-parity findings.  The candidate covers every selected declaration and
macro in `arch/x86/include/asm/orc_lookup.h` for the frozen x86_64
configuration without changing the linker-defined-symbol contract.

## Evidence inspected

- `rewrite/SCOPE.tsv` row `S000623` maps the x86_64 header to
  `src/arch/x86/include/asm/orc_lookup.rs`, classifies it `RUST_TRANSLATE`, and
  records its sole selected consumer as `arch/x86/kernel/unwind_orc.o`.
  `rewrite/FILE_MAP.tsv` and `rewrite/metadata/header_closure.tsv` identify the
  same C consumer and its frozen x86_64 command/evidence.
- `rewrite/SYMBOLS.tsv` rows 28476–28486 select the include guard, both
  `LINKER_SCRIPT` condition boundaries, `LOOKUP_BLOCK_ORDER`,
  `LOOKUP_BLOCK_SIZE`, `LOOKUP_START_IP`, `LOOKUP_STOP_IP`, `orc_lookup`, and
  `orc_lookup_end`.  `rewrite/LIFETIMES.tsv` rows 14749–14750 and
  `rewrite/ABI.tsv` rows 14840–14841 record the two external globals.  The
  candidate supplies both symbols, both constant expressions, and both address
  expression macros.
- The frozen config has `CONFIG_64BIT=y`, `CONFIG_X86_64=y`, and
  `CONFIG_UNWINDER_ORC=y`.  The task provenance revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df` agrees with `vendor/linux.SHA`
  and `rewrite/PHASE0_IDENTITY.tsv`; SPDX and x86_64 task provenance agree with
  the selected source.

## Normal C-consumer branch

- Upstream lines 26–27 declare `extern unsigned int orc_lookup[]` and
  `orc_lookup_end[]`.  The candidate declares same-named mutable foreign `u32`
  anchors.  On the selected x86_64 target, `unsigned int` is 32 bits; an anchor
  declaration preserves the linker symbol and the C array-base address used for
  indexing and endpoint subtraction without inventing a Rust allocation.
- Upstream lines 21–22 define `8` and `(1 << 8)`, both C `int` expressions.
  The candidate's `i32` constants retain that width, signedness, and value.
- Upstream lines 29–30 cast the addresses of the `char[]` linker symbols
  `_stext` and `_etext` to `unsigned long`.  `asm-generic/sections.h:35`
  supplies those declarations, and x86_64 makes `unsigned long` 64 bits.  The
  candidate's `addr_of!` macros produce the same symbol addresses as `usize`
  without forming Rust references.  `unwind_orc.c:217–229` and `355–375` use
  precisely these values for range tests, lookup-block calculation, indexing,
  and initialization.

## LINKER_SCRIPT branch and linker contract

- `scripts/Makefile.build:443–448` preprocesses `.lds.S` with
  `-DLINKER_SCRIPT`.  Thus upstream header lines 24–32 intentionally leave only
  the two numeric macros available to `arch/x86/kernel/vmlinux.lds.S`.
- With `CONFIG_UNWINDER_ORC=y`, `asm-generic/vmlinux.lds.h:862–882` uses
  `LOOKUP_BLOCK_SIZE` to reserve `.orc_lookup`, defines `orc_lookup` at its
  beginning, and defines `orc_lookup_end` at its one-past-end address.
  `vmlinux.lds.S:137`, `167`, and `209` define the `_stext`/`_etext` boundary
  symbols and expand the ORC table section.  The Rust module is not an input to
  that C-preprocessed linker-script path, so its normal-code FFI declarations
  do not leak into the `LINKER_SCRIPT` branch; the original selected linker
  script retains the upstream expressions and symbol definitions.

No compiler, formatter, linker, test, or runtime command was used.
