# Rust review — S012503 (attempt 2)

## Verdict

Rejected pending applier resolution.  This was a manual source review only; no
compiler, formatter, linker, test, or runtime tool was run.

## Evidence inspected

- `vendor/linux/include/asm-generic/audit_write.h:1-25`
- its direct selected inclusion contexts:
  `arch/x86/kernel/audit_64.c:18-20`, `arch/x86/ia32/audit.c:16-18`,
  `lib/audit.c:17-19`, and `lib/compat_audit.c:17-19`
- the selected header-closure and dependency records for `S012503`, including
  the x86_64 consumers (the two x86 audit TUs) and AArch64 consumers (the two
  `lib` audit TUs)
- `src/include/asm-generic/audit_write.rs:1-69`

The four literal sequences themselves are accurate for the frozen contexts:
x86_64 native has 22 entries, x86 IA32 has 23, AArch64 native has 16
(including the duplicate `truncate`/`truncate64` and
`ftruncate`/`ftruncate64` values), and AArch64 compat has 25.  In each C
consumer the following `~0U` is outside this header fragment, so it is
correctly not counted as a header entry.

## Findings

### R1 — high: `#[macro_export]` exposes all architecture-specific fragments in every build

`audit_write.h` defines no C symbol or reusable macro.  It is a textual
initializer fragment, interpreted only after the including TU has selected its
own `asm/unistd*` definitions.  Thus the x86 native fragment is available only
to `audit_64.c`, the x86 IA32 fragment only to `ia32/audit.c`, and the analogous
selection on AArch64 is made by the two `lib` TUs.

In contrast, lines 11, 27, 43, and 59 use `#[macro_export]` without any target
or inclusion-context gating.  Each build therefore exports crate-root Rust
macros for both architectures and both ABI variants.  That adds global API
surface and lets an unrelated or wrong-architecture consumer select a syscall
table which the corresponding C preprocessing context never exposes.  This is
a scope/visibility and architecture-selection difference, not merely a naming
difference.

The applier needs a source-local representation whose availability follows the
actual translated consumer/context (or an explicitly frozen, context-bound
interface), rather than global exports of every architecture variant.

### R2 — high: the callback-only macro contract is not the C fragment's direct initializer contract

At every selected C inclusion site, preprocessing inserts the header tokens
directly between the array's `{` and the caller-owned `~0U`.  The candidate
instead requires a new `$consumer:ident` callback macro and transfers all
array-expression construction to that callback.  It cannot be invoked where
the C include occurs, and it imposes an identifier-only callback interface that
does not exist in the pinned source.

The surrounding callers have not yet been translated, so no current Rust
consumer proves that this added protocol constructs an `unsigned int`-width
array with the immediately following `u32::MAX` sentinel, preserves each
caller's static versus external visibility, and selects the right ABI list.
Comments assigning those obligations to a consumer do not establish the
required mapping.  If a callback/X-macro representation is unavoidable in
Rust, it must be a documented context-bound translation mechanism and be
consumed by each of the four mapped caller paths so that the full C initializer
semantics, order, type width, and sentinel placement remain explicit.

## Other checks

- The nested `audit_dir_write.h` entries are present at the front of every
  candidate sequence and retain order.
- All values are suffixed `u32`, consistent with the selected C consumers'
  `unsigned`/`unsigned int` element type; no truncating cast was introduced.
- The source provenance fields name the pinned file, revision, architecture
  class, and task.  The Rust SPDX `GPL-2.0-only` is consistent with the
  project-required provenance form for this GPL-2.0 Linux header; no branding
  delta was observed.

