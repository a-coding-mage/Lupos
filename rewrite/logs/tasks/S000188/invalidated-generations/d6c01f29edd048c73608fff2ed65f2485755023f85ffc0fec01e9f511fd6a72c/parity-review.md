# Parity review — S000188

## Verdict

Reject: the candidate preserves a few resulting numbers, but it does not
preserve this header's active preprocessor-selection and generated-header
inclusion semantics.

## Findings

1. **Blocking — all fourteen `__ARCH_WANT_*` definitions were reduced to unit
   constants, so none can select downstream declarations or generated syscall
   entries.** `arch/arm64/include/asm/unistd.h:5-29` defines the twelve
   compatibility wants under active `CONFIG_COMPAT` and defines
   `__ARCH_WANT_SYS_CLONE` and `__ARCH_WANT_NEW_STAT` unconditionally.  These
   are presence macros, not values.  The frozen AArch64 configuration has
   `CONFIG_COMPAT=y` (line 498), so the first twelve are active in the approved
   build.  The `()` constants at candidate lines 13-35 neither participate in
   conditional translation nor cause the selected items to exist.  Concrete
   consumers include `include/linux/syscalls.h:532,1059` for
   `__ARCH_WANT_COMPAT_STAT64`, `include/linux/compat.h:850,854` for the old
   signal interfaces, and the generated generic syscall header's
   `__ARCH_WANT_NEW_STAT` branches at `include/uapi/asm-generic/unistd.h:209,
   887,905`.  Resolve by representing the active frozen conditional selections
   in the translated consumers/generated binding, with a mechanically bound
   CONFIG_COMPAT selection mechanism, rather than by exporting marker values.

2. **Blocking — the literal include of generated `asm/unistd_64.h` is omitted.**
   The source's line 31 inclusion edge is recorded in
   `rewrite/metadata/header_include_edges.tsv:600` to
   `generated/aarch64/arch/arm64/include/generated/uapi/asm/unistd_64.h`
   (S012326, BUILD_METADATA).  That generated header exposes the generic
   syscall-number and syscall-selection macro surface; it is not equivalent to
   one final count.  In particular, it supplies `__NR_syscalls` (after the
   generic header's final `#undef`/definition at
   `include/uapi/asm-generic/unistd.h:866-867`) and the `__NR_*` definitions
   and aliases selected by the preceding wants.  Candidate line 43 has no
   generated-artifact binding or re-export and therefore loses the include's
   API and selection effects.  Restore an exact, Phase-0-bound representation
   of that generated dependency and its selected exported definitions.

3. **Major — `NR_syscalls` is not an alias and changes the source expression's
   type contract.** Source line 33 defines `NR_syscalls` as the parenthesized
   token alias `(__NR_syscalls)`; candidate line 43 invents `usize = 472`.
   The generated source is the authority for the value and `__NR_syscalls` is
   a C integer macro (472), while `usize` is an AArch64 pointer-width type.
   This also erases the named generated dependency needed by native syscall
   table users (`arch/arm64/kernel/sys.c:60-61`,
   `arch/arm64/kernel/syscall.c:140`) and seccomp
   (`arch/arm64/include/asm/seccomp.h:23`).  Define/re-export the generated
   `__NR_syscalls` with its faithful integer representation and make
   `NR_syscalls` the corresponding alias expression; do not duplicate the
   generated count as an independently maintained `usize` literal.

4. **Major — the candidate makes the `CONFIG_COMPAT` content unconditionally
   available without a source-level configuration binding.** The explanatory
   comment at line 11 is not a configuration selection mechanism.  The frozen
   configuration currently selects the branch, but the mapping must record and
   enforce that fact in the generated/config-bound translation so the twelve
   compatibility SVC and selection definitions arise from the selected branch
   and disappear with it.  This is especially material because the private
   SVC numbers are consumed by `arch/arm64/kernel/sys_compat.c:87,90,108`.

## Items checked without findings

- Immutable provenance identifies the correct source, revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, architecture, and task.
- Under the active compatibility branch, the four private SVC expressions are
  numerically preserved: base `0x0f0000`, cacheflush `base + 2`, set_tls
  `base + 5`, and end `base + 0x800`.  `i32` is compatible with their C
  integer-constant-expression uses in the inspected `int scno` switch/range
  context; this does not remedy the missing conditional mechanism above.

No compiler, formatter, linker, test, runtime, or diagnostic tool was used.
