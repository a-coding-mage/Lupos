# Parity review — S012501

## Scope and evidence

- Reviewed candidate: `src/include/asm-generic/audit_read.rs`.
- Oracle: `vendor/linux/include/asm-generic/audit_read.h` at lines 1–20.
- Frozen task mapping: `rewrite/SCOPE.tsv` row `S012501` and the matching
  `rewrite/FILE_MAP.tsv` entries.  The selected consumers are the native and
  compat audit-class initializers: `arch/x86/kernel/audit_64.c:13-16`,
  `arch/x86/ia32/audit.c:21-24`, `lib/audit.c:12-15`, and
  `lib/compat_audit.c:12-15`.
- Frozen configuration evidence: x86_64 enables `CONFIG_AUDIT_ARCH` and
  `CONFIG_IA32_EMULATION`; AArch64 enables `CONFIG_AUDIT_GENERIC`,
  `CONFIG_AUDIT_ARCH_COMPAT_GENERIC`, and `CONFIG_AUDIT_COMPAT_GENERIC`.
- The raw numbers and order in all four candidate sequences do agree with the
  selected syscall namespaces: x86_64 native, x86 IA32, AArch64 native
  asm-generic, and AArch32 compat.  In particular, the AArch64-native form
  correctly omits `__NR_readlink`, and the two `*xattrat` numbers are 464 and
  465 where their namespaces define them.  Source evidence is
  `include/uapi/asm-generic/unistd.h:53-64,166-167,206-207,846-849`,
  `arch/x86/entry/syscalls/syscall_{64,32}.tbl`, and
  `arch/arm64/tools/syscall_32.tbl`.

## Findings

### P1 — the candidate replaces contextual preprocessor semantics with four unconditional public APIs

`audit_read.h` is deliberately an unnamed initializer-token fragment, not a
header-defined function, object, or macro API.  Its four selected `#ifdef`s at
lines 2, 7, 13, and 18 are evaluated in the including translation unit against
that unit's active `__NR_*` namespace.  The inventory records all eight
conditional records for both architectures in `rewrite/SYMBOLS.tsv` under
S012501.

The candidate instead declares four unconditional `#[macro_export]`
macros (lines 11-64), each containing a permanently chosen number list.
`#[macro_export]` publishes those new names at crate root for every build;
therefore an x86 build gains AArch64/AArch32 macro APIs and an AArch64 build
gains x86/IA32 macro APIs.  No equivalent exported names exist in the Linux
source, and the original conditional selection no longer follows the active
syscall namespace or the frozen configuration at expansion time.

This is not merely a cosmetic representation change: each consumer must now
manually choose an invented architecture/ABI-specific exported macro, whereas
the C include automatically emits the correct initializer tokens from the
consumer's `asm/unistd*.h` namespace.  It also permits a selected consumer to
choose the wrong profile with no source-level guard.

Required resolution: preserve the header's fragment role and make every
selected translated consumer receive only its own frozen namespace's entries,
with configuration/architecture gating matching its C inclusion context.  Do
not leave new unconditional crate-root macro APIs as a substitute for that
selection.  If Rust requires helper macros, their visibility and selection must
be constrained to the corresponding consumer/context and explicitly mapped.

### P1 — the callback interface is not equivalent to C initializer-token expansion

At each inclusion site, Linux expands this file directly inside an `unsigned`
array initializer, after which that *same initializer* contains the adjacent
`~0U` sentinel (`audit_64.c:13-16`, `ia32/audit.c:21-24`, `lib/audit.c:12-15`,
and `compat_audit.c:12-15`).  The candidate's `$consumer:ident` callback
interface (lines 12-18, 26-32, 42-48, and 57-63) cannot itself expand as the
initializer fragment.  It requires a separately invented callback macro,
restricts it to a bare identifier, and does not establish the receiving array's
type or the required sentinel adjacency.

Its comments state those requirements but source comments do not enforce the
Linux behavior.  This is an omitted mapping of the header's operative
inclusion contract.  Resolve it with an interface that can be used as the
actual selected initializer content (including the `u32`/`unsigned` array and
`!0u32` sentinel at the same consumer location), rather than an unvalidated
out-of-band callback convention.

## Result

Reject pending correction of both P1 findings.  No compiler, formatter,
analyzer, test, or runtime command was used.
