# Parity review — S000191

## Verdict

**REJECT: source parity is not established.** The candidate retains the value
`__VDSO_PAGES == 4`, the SPDX identifier, and the ARM 2012 copyright notice,
and its `wrapping_add` models the `unsigned long` addition only after a caller
has already supplied the correct address and offset.  It does not preserve the
operative generated-header, macro, externally named-symbol, or
C-versus-assembler interface required by the pinned source.

## Frozen evidence consulted

- Oracle: `vendor/linux/arch/arm64/include/asm/vdso.h:5-24`.
- Frozen aarch64 configuration: `rewrite/configs/aarch64/frozen.config:500-501`
  selects `CONFIG_COMPAT_VDSO=y` and `CONFIG_THUMB2_COMPAT_VDSO=y`.
- Generated Phase 0 evidence:
  `rewrite/metadata/aarch64/generated-headers-include-generated.tar` member
  `include/generated/vdso-offsets.h` contains exactly
  `#define vdso_offset_sigtramp 0x08d0`; the generated-header edge is recorded
  in `rewrite/metadata/header_include_edges.tsv:605`.  The generator is
  `vendor/linux/arch/arm64/kernel/vdso/gen_vdso_offsets.sh:13-16`, and its
  Makefile rule writes this header from the native vDSO symbol table at
  `vendor/linux/arch/arm64/kernel/vdso/Makefile:69-74`.
- Selected consumers: `arch/arm64/kernel/signal.c:1481-1486` invokes
  `VDSO_SYMBOL(current->mm->context.vdso, sigtramp)`; `alternative.c:213-214`
  treats `vdso_start` directly as a link-address object; `vdso.c:45-57` stores
  all four boundary addresses and conditionally uses the two compat addresses.
- Assembly/link evidence: both vDSO linker scripts include this header
  (`vdso/vdso.lds.S:11-16`, `vdso32/vdso.lds.S:11-16`).  The wrappers define
  global `vdso_start`/`vdso_end` and `vdso32_start`/`vdso32_end`
  (`vdso-wrap.S:14-20`, `vdso32-wrap.S:11-17`); their selected addresses are
  present in `rewrite/metadata/aarch64/System.map:62395-62399`.  Kernel Kbuild
  selects `vdso32-wrap.o` for this frozen config
  (`arch/arm64/kernel/Makefile:73-78`).

## Findings requiring resolution

### P1 — `VDSO_SYMBOL` no longer implements the generated token-pasted macro

Oracle `VDSO_SYMBOL(base, name)` performs token pasting to select
`vdso_offset_##name` from the generated header; `name` is an identifier, is
not evaluated, and only `base` is evaluated once.  The candidate instead makes
the second argument an arbitrary evaluated Rust expression and requires an
outside caller to pass a numeric offset.  No generated `vdso_offset_sigtramp`
binding is present anywhere under `src/`, so the selected signal consumer
cannot retain its oracle invocation/selection contract.  The candidate's
comment saying that a binding “has supplied” the value is not an
implementation.  It also narrows `base` to `*const u8`, whereas the oracle
first casts its caller-provided value to `unsigned long` and returns `void *`.

Resolve by preserving a generated-offset binding derived from the frozen
`vdso-offsets.h` evidence and a call surface that maps the symbolic `name` to
the matching generated offset without evaluating it.  The resulting pointer
expression must retain AArch64 `unsigned long` wrapping and a single evaluation
of `base`, while being usable for the oracle consumer's `void *` context.

### P1 — external boundary declarations were substituted with private anchors
and functions

The oracle declares four externally linked incomplete `char` arrays named
`vdso_start`, `vdso_end`, `vdso32_start`, and `vdso32_end`.  They are address
objects, not calls.  The candidate creates private Rust statics with different
Rust identifiers and exposes four `pub(crate)` functions bearing the oracle
names.  This changes direct-object use such as the `alternative.c` cast and
the `vdso.c` address initializers into calls, and does not provide the external
symbol declarations under their original Rust-facing names.  `#[link_name]`
on a private zero-size anchor preserves an internal lookup only; it is not a
faithful replacement API for the header declarations.

Resolve by exposing raw external address anchors for all four linker symbols
with their source-level names and address-only/incomplete-array semantics,
then have consumers take their addresses directly.  Do not replace an oracle
object interface with helper functions.  Retain both compat declarations: the
header declares them unconditionally and the frozen config also supplies their
definitions.

### P1 — the `__ASSEMBLER__` split is absent

Oracle keeps `__VDSO_PAGES` outside `#ifndef __ASSEMBLER__`, while excluding
the generated C offset header, C statement-expression macro, and C extern
declarations from assembler/linker-script inclusion.  Both selected native and
compat linker scripts include `<asm/vdso.h>`, so this is operative interface,
not a cosmetic include guard.  The Rust-only candidate provides no equivalent
assembly-safe surface or documented preserved mapping for this split, and its
unconditional Rust FFI/macro declarations cannot serve those linker-script
consumers.

Resolve with a path-preserving arrangement that leaves the selected original
assembly/linker-script view with exactly the macro-only `__VDSO_PAGES` contract
and confines generated offsets, expression macro, and external declarations
to the non-assembler view.  No generated C declarations may leak into the
assembler path.

## Checks with no finding

- Candidate provenance identifies the pinned source/revision, task, and
  `aarch64` architecture.
- `pub const __VDSO_PAGES: usize = 4` has the correct numeric value for a
  Rust consumer, but does not by itself satisfy the assembler/preprocessor
  interface above.
- No unauthorized Lupos branding, test code, placeholder, or license/copyright
  text change was found in the candidate.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or runtime
command was used in this review.
