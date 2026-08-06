# Applier resolution — S016327

## Decision

Accepted without source amendment.  `src/include/uapi/linux/personality.rs`
is a complete fresh translation of the selected public constant interface in
`vendor/linux/include/uapi/linux/personality.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Independent source reconciliation

1. **Anonymous flag enum (Linux lines 11–25): resolved.**  This is an
   anonymous enum that creates no named C type, object, linkage, layout, or
   lifetime-bearing storage.  Each enumerator is an integer constant
   expression and all values fit signed 32-bit `int`; the Rust `pub const ...:
   i32` items at lines 12–26 preserve the names and values exactly.
2. **`PER_CLEAR_ON_SETID` (Linux lines 31–34): resolved.**  The candidate at
   Rust lines 32–33 retains the same four `int` operands in source order:
   `READ_IMPLIES_EXEC | ADDR_NO_RANDOMIZE | ADDR_COMPAT_LAYOUT |
   MMAP_PAGE_ZERO`.  Its value is `0x0740000`, representable in `i32`; no
   signed overflow, truncation, shift, complement, or side effect occurs.
   In the pinned consumers, C's usual arithmetic conversions apply only when
   this constant is combined with unsigned personality state; recording the
   constant itself as `i32` preserves its source expression category rather
   than falsely recasting the public macro as an unsigned object.
3. **Anonymous personality enum (Linux lines 42–67): resolved.**  The
   candidate at Rust lines 41–67 preserves every enumerator, literal, and OR
   operand.  The largest expression is `0x410000e`, below `INT_MAX`, so each
   source enumerator and corresponding Rust constant has signed 32-bit
   integer value semantics.
4. **C inclusion guard (Linux lines 2–3 and 70): resolved.**  It solely
   controls C preprocessing and has no Rust runtime, ABI, ownership, or
   storage analogue.  The source has no configuration-selected branch beyond
   that guard for either frozen architecture.
5. **Review reports: resolved.**  The parity report and Rust review each
   reported no finding.  I independently checked their conclusions against
   the complete pinned header and the relevant pinned consumers:
   `include/linux/personality.h`, `fs/exec.c`, `fs/binfmt_elf.c`,
   `security/commoncap.c`, `arch/x86/kernel/process_64.c`,
   `arch/x86/mm/mmap.c`, `arch/arm64/kernel/process.c`, and
   `arch/arm64/include/asm/elf.h`.  No public name, expression, ABI, or
   behavior is omitted or changed.

## Semantic-record closure

All twelve S016327 `SYMBOLS.tsv` rows are `COMPLETE`: the guard is marked
C-preprocessor-only, the macro is recorded as the signed-`i32` OR expression,
and both anonymous enums are recorded as constant-expression-only.  The four
`ABI.tsv` rows now state that neither anonymous enum declares a named/exported
type or stored layout.  The four `LIFETIMES.tsv` rows now state that the enums
have no storage, ownership, lifetime, locking, RCU, or refcount family.  Each
closure cites the pinned source and candidate location for both x86_64 and
aarch64 records.

No compiler, formatter, linker, test, emulator, debugger, runtime command,
or benchmark was run.
