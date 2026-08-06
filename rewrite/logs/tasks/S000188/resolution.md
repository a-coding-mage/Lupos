# Resolution — S000188

## Outcome

**BLOCKED.** The candidate is rejected unchanged.  The frozen scope has no
path-preserving Rust representation of the generated header included by this
file, nor a frozen configuration-selection interface through which its
preprocessor selectors can act on translated consumers.  Adding either one to
this single-file task would cross the immutable Phase 0 scope; replacing them
with constants would change the upstream mechanism and API.

## Independent source basis

- `vendor/linux/arch/arm64/include/asm/unistd.h:5-29` makes twelve
  `__ARCH_WANT_*` selectors conditional on `CONFIG_COMPAT`, then defines
  `__ARCH_WANT_SYS_CLONE` and `__ARCH_WANT_NEW_STAT` unconditionally.  The
  frozen AArch64 configuration has `CONFIG_COMPAT=y`.
- Those are C macro-definedness selectors, not values.  They direct downstream
  preprocessing, including `include/linux/syscalls.h:532,1059`,
  `include/linux/compat.h:850,854`, and
  `include/uapi/asm-generic/unistd.h:209,887,905`.  Rust unit constants cannot
  reproduce definedness or make an includer select corresponding declarations.
- `unistd.h:31` literally includes generated
  `asm/unistd_64.h`.  Phase 0 records that edge in
  `rewrite/metadata/header_include_edges.tsv` and classifies its target as
  `S012326`, `BUILD_METADATA`, with no destination Rust path in
  `rewrite/SCOPE.tsv`.  No scoped Rust generated-header binding exists under
  `src/arch/arm64/include/asm/`; this task's `unistd.rs` is the only path in
  that family.
- The generated generic syscall surface supplies selected `__NR_*` definitions
  and aliases before defining `__NR_syscalls` as `472`
  (`vendor/linux/include/uapi/asm-generic/unistd.h:866-867`).  The source then
  defines `NR_syscalls` as `(__NR_syscalls)` at `unistd.h:33`.  Candidate
  `NR_syscalls: usize = 472` both drops that dependency/namespace and invents a
  pointer-sized type rather than preserving the C integer macro expression.

## Review-finding dispositions

1. **Parity 1 / Rust R1 — accepted.** Candidate lines 15-26 and 34-35 are
   not a faithful representation of the fourteen selector macros.  A proper
   resolution requires a frozen, mechanically bound configuration-selection
   mechanism implemented by the translated consumers; none is assigned to
   S000188.
2. **Parity 2 / Rust R2 — accepted.** Candidate omits the operative generated
   header inclusion.  S012326 is `BUILD_METADATA`, has no Rust destination,
   and its complete selected macro namespace cannot be reconstructed in this
   one source file without inventing a new generated binding or hardcoding an
   incomplete substitute.
3. **Parity 3 / Rust R3 — accepted.** Candidate's standalone `usize` literal
   is neither the source alias nor its integer-expression contract.  It cannot
   be corrected faithfully until the generated `__NR_syscalls` binding exists.
4. **Parity 4 — accepted.** The comment stating that `CONFIG_COMPAT` is
   enabled is not an operative selection mechanism.  The frozen configuration
   establishes that the C branch is active, but does not itself provide a
   translated Rust selector interface.

## Semantic-record closure

The task's `PENDING_REVIEW` selector and generated-alias records cannot be
closed as `COMPLETE`: their exact behavior depends on an absent, out-of-task
generated-header/configuration binding.  This is a scope/translation-boundary
blocker, not permission to retain value constants as a substitute.

No compiler, formatter, linker, test, runtime, or diagnostic tool was used.
