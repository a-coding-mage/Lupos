# Applier resolution — S012503 (attempt 2)

## Disposition

**BLOCKED.** The candidate cannot be accepted, and the frozen mapping does not
provide a source-local Rust representation for this C textual initializer
fragment that preserves all four selected inclusion contexts.

## Source evidence

The pinned source `include/asm-generic/audit_write.h:1-25` is itself a token
fragment.  Its exact `GPL-2.0` SPDX line is followed by a textual include of
`asm-generic/audit_dir_write.h`, then syscall-number tokens selected by the
including translation unit's `asm/unistd*` definitions.  It declares neither a
C object nor a C macro.

The selected direct consumers place the fragment between their own array
braces and their own following `~0U` sentinel:

- `arch/x86/kernel/audit_64.c:18-21` — private native `write_class`;
- `arch/x86/ia32/audit.c:16-19` — externally visible IA32 `ia32_write_class`;
- `lib/audit.c:17-20` — private AArch64-native `write_class`; and
- `lib/compat_audit.c:17-20` — externally visible AArch32-compatible
  `compat_write_class`.

The frozen task row maps only this common header to
`src/include/asm-generic/audit_write.rs`, with S012500 as its sole dependency.
The Phase 0 mapping and the task's semantic records do not specify a
context-bound Rust fragment/include form, nor an interface owned by the four
as-yet-untranslated consumer paths.

## Review findings

1. **Parity P1 — accepted.** Candidate lines 15-17, 31-33, 47-48, and 63-65
   duplicate the S012500 directory-write entries rather than compose the
   source's `#include <asm-generic/audit_dir_write.h>` boundary.  Replacing
   those literals with the existing dependency's exported callback macros
   would retain the separate, unconstrained callback protocol and would not
   restore the C textual-inclusion scope.
2. **Parity P1 — accepted.** Candidate line 1 says `GPL-2.0-only`, whereas
   upstream line 1 says `GPL-2.0`.  Retaining the exact upstream SPDX token is
   required, but that narrow correction cannot resolve the representation
   failure.
3. **Rust R1 — accepted.** `#[macro_export]` exposes all four
   architecture/ABI expansions at crate root.  The C header's content is only
   available after a particular translation unit selects one syscall namespace;
   it does not expose an architecture-agnostic global API.
4. **Rust R2 — accepted.** The `$consumer:ident` callback protocol is not a
   direct Rust analogue of the C tokens inserted into a caller's initializer.
   No selected translated consumer currently binds the protocol to the source
   array visibility, element width, ordering, and immediately following
   sentinel.

## Why no source change was applied

The only apparent changes are either (a) corrected but still globally exported
macros, (b) another unreviewed callback/X-macro convention, or (c) duplicating
the fragment in future consumer translations.  Each changes the pinned
source's ownership or context-selection behavior.  Introducing a private
module API or editing the four consumer task destinations would also exceed
this one-file task and lack a frozen representation/mapping decision.

An exact resolution requires a scope/translation-design decision that maps
textual initializer headers and their direct consumers together, specifies how
the S012500 composition boundary is retained, and binds each exported or
private Rust item to the corresponding frozen architecture/ABI context.  Until
then, the candidate remains rejected and no semantic `PENDING_REVIEW` record
for this task is closed.

No compiler, formatter, linker, test, emulator, debugger, or benchmark was
used for this resolution.
