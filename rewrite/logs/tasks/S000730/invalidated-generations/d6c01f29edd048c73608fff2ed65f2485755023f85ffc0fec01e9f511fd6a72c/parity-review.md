# Parity review — S000730 (slot 1)

Reviewed independently against the complete pinned
`vendor/linux/arch/x86/include/asm/trapnr.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 configuration,
and the Phase 0 scope/symbol/header-closure records.

## Result

No source-parity findings.  `src/arch/x86/include/asm/trapnr.rs` is a complete
one-to-one representation of the selected header's operative constant contract.

## Exhaustive comparison

- The upstream include guard is correctly absent from the Rust item surface:
  it contributes no type, storage, linkage, or runtime ABI.
- All eight unconditional event-type macro names and values match exactly:
  `EVENT_TYPE_EXTINT=0`, `EVENT_TYPE_RESERVED=1`, `EVENT_TYPE_NMI=2`,
  `EVENT_TYPE_HWEXC=3`, `EVENT_TYPE_SWINT=4`, `EVENT_TYPE_PRIV_SWEXC=5`,
  `EVENT_TYPE_SWEXC=6`, and `EVENT_TYPE_OTHER=7`.
- All twenty-four unconditional trap-number macro names and values match
  exactly: `X86_TRAP_DE=0`, `DB=1`, `NMI=2`, `BP=3`, `OF=4`, `BR=5`, `UD=6`,
  `NM=7`, `DF=8`, `OLD_MF=9`, `TS=10`, `NP=11`, `SS=12`, `GP=13`, `PF=14`,
  `SPURIOUS=15`, `MF=16`, `AC=17`, `MC=18`, `XF=19`, `VE=20`, `CP=21`,
  `VC=29`, and `IRET=32`.  The candidate retains the `X86_TRAP_` prefix for
  each abbreviated name in this list.
- Every upstream replacement list is an unsuffixed decimal integer literal;
  on the frozen `x86_64-linux-gnu` target the values fit `int`.  The candidate
  exposes each as `pub const ...: i32`, preserving the signed 32-bit value
  category at Rust translation use sites.  No changed suffix, width, sign,
  expression, flag, or conditional definition was introduced.
- The header has no includes, types, functions, objects, linkage, layout,
  locking, allocation, cleanup, or configuration-conditioned declarations.
  `CONFIG_X86_64=y` and `CONFIG_X86_FRED` is unset, but neither changes the
  unconditional macro surface.  The disabled FRED consumer is therefore not
  an excuse to remove any event type.
- `rewrite/metadata/header_closure.tsv` records 234 selected consumers, among
  them retained Linux assembly.  The pinned assembly obtains these immediate
  tokens by retaining and preprocessing its original C header; it does not
  require a Rust exported object or symbol.  The Rust constants correctly add
  neither.  Direct context confirms use in both C (`arch/x86/entry/common.c`)
  and assembly (`arch/x86/entry/entry_64.S`), as well as `asm/fred.h`,
  `asm/idtentry.h`, and `asm/vmx.h`.
- The candidate has the required immutable source/revision/architecture/task
  provenance, contains no test configuration, placeholder, unsafe code,
  runtime substitute, or unallowlisted branding change.  Its required
  `GPL-2.0-only` Rust provenance line follows the project-mandated fresh-source
  header form.

## Record closure for the applier

The Phase 0 `SYMBOLS.tsv` rows for this task are still `PENDING_REVIEW`.  For
the final task resolution, close them with the facts above: all 32 operative
macros are unconditional signed-`int` decimal constants; the two guard rows
and `_ASM_X86_TRAPNR_H` have no Rust runtime/ABI counterpart; and there are no
per-file ABI, lifetime, ownership, locking, RCU, refcount, or cleanup records
to add.  This is a required workflow closure step, not a candidate defect.
