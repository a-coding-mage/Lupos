# Resolution — S012501

Pipeline: P02  
Role: applier  
Disposition: **BLOCKED**

## P1 / R1 — exported callback macros are not an equivalent mapping

**Accepted.** `include/asm-generic/audit_read.h` (lines 2–20) declares no
symbol, object, or macro and has no include guard.  It is a context-dependent
comma-terminated initializer-token fragment.  Each selected C owner expands
it directly between `{` and the caller-owned `~0U` sentinel:

- `arch/x86/kernel/audit_64.c:13–16` (`static unsigned read_class[]`),
- `arch/x86/ia32/audit.c:21–24` (`unsigned ia32_read_class[]`),
- `lib/audit.c:12–15` (`static unsigned read_class[]`), and
- `lib/compat_audit.c:12–15` (`unsigned int compat_read_class[]`).

The candidate's four `#[macro_export]` callback macros create new crate-root
APIs, require a separately invented consumer macro, and expose all ABI
profiles to every build.  No pinned source, scope, ABI, lifetime, or file-map
record authorizes those exported names or that callback protocol.  They cannot
substitute directly for the C initializer fragment.

## P1 / R2 — caller-selected profile and sentinel/comma contract

**Accepted.** The header evaluates `__NR_readlink`, `__NR_listxattrat`,
`__NR_getxattrat`, and `__NR_readlinkat` in the including translation unit.
The frozen source establishes four distinct selected contexts: x86_64 native,
x86 IA32, AArch64 native, and AArch32 compat.  The candidate's sequences agree
numerically with those contexts, but its function-like callback macro parameter
does not bind a sequence to its owner or preserve direct placement before the
same array initializer's `~0U` terminator.  The consuming scan in
`kernel/auditfilter.c:168–187` requires that terminator.

The `S012501` records in `rewrite/SYMBOLS.tsv` retain all sixteen conditional
facts as `PENDING_REVIEW`; `rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` have
no `S012501` owner-contract record.  Phase 0 provides header-closure evidence
for the four C consumers, but no frozen Rust owner-side mapping, private
module visibility rule, or mechanism representing inclusion-time expression
tokens.  Creating one here would both invent an API and require edits to four
other queued destination files, outside this leased task.

## Blocking requirement

Reopen the scope/Phase 0 gate to add an explicit, frozen mapping for this
context-dependent fragment: each selected Rust owner must receive its exact
`u32` entry sequence in its own source-local/static array with `u32::MAX` in
the same initializer, and the mapping must bind its ABI/architecture rather
than export a selectable cross-architecture macro.  The records must also
resolve the sixteen conditional facts.  Until then, exact Rust parity is not
established and the candidate is not accepted.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or runtime
tool was used.
