# S013611 parity review (slot 1)

## Verdict

REJECT.  `src/include/linux/compiler-version.rs` is not a semantic translation
of the pinned `include/linux/compiler-version.h`.  Its immutable provenance is
correct (`425f94c2954b1fe80ebdbf9b29854e89750355df`, `common`, `S013611`) and
it contains no test or placeholder token, but its only executable item is an
unrelated public Rust constant while the upstream file's operative behavior is
build-system/preprocessor behavior.

## Findings

### P1 — The mandatory compiler-version rebuild dependency is replaced by a comment

Pinned source lines 8–14 deliberately put the literal `CONFIG_CC_VERSION_TEXT`
in a header force-included into every C compilation.  `scripts/basic/fixdep.c`
lines 73–85 and 189–204 establish that fixdep scans every prerequisite text,
including comments, and emits an `include/config/<symbol>` dependency.  Thus a
compiler-version Kconfig update touches `include/config/CC_VERSION_TEXT` and
causes affected translation units to rebuild.

The candidate merely repeats this literal in a Rust comment (lines 11–15).  It
does not establish any dependency on `include/config/CC_VERSION_TEXT`, expose a
Kbuild/fixdep input, or otherwise make a compiler-version change rebuild the
translated consumers.  A comment is not a replacement for the required build
edge.  The applier must preserve this exact dependency through the Rust build
integration selected for the translation rather than documenting it.

### P1 — Forced inclusion and direct-include rejection are absent

Pinned `vendor/linux/Makefile` lines 584–592 add
`-include $(srctree)/include/linux/compiler-version.h` to `USERINCLUDE`; the
frozen representative commands
`rewrite/kbuild/x86_64/arch/x86/entry/.common.o.cmd` and
`rewrite/kbuild/aarch64/arch/arm/xen/.enlighten.o.cmd` both contain the exact
absolute `-include` argument.  The header therefore defines
`__LINUX_COMPILER_VERSION_H` before each C source is processed.  Pinned lines
3–6 then deliberately fail any later/direct include with the exact diagnostic
that this header is supplied by the build system.

Candidate lines 7–17 have no forced-inclusion mechanism and do not reject a
source-level/direct inclusion.  The claim that ordinary Rust module inclusion
supplies an equivalent single-definition property is false: it neither makes
the item globally preincluded for each consumer nor preserves the explicit
misuse failure.  The added public `const __LINUX_COMPILER_VERSION_H: ()` is a
new Rust value, not the stateful C preprocessor guard/error protocol.

### P1 — The three conditional generated-header dependency paths are discarded rather than represented

Pinned lines 23–25, 32–34, and 42–44 make `gcc-plugins.h`,
`randstruct_hash.h`, and `integer-wrap.h` tree-wide dependency inputs whenever
`GCC_PLUGINS`, `RANDSTRUCT`, or `INTEGER_WRAP` is supplied by the build.
For the current frozen union, this review verified all three macro flags occur
zero times in each architecture's authoritative `rewrite/kbuild/*` command
records; both frozen configs select `CONFIG_RANDSTRUCT_NONE=y` and have
`CONFIG_UBSAN` disabled; and none of the three generated headers exists under
either authoritative `rewrite/kbuild/` tree.  Therefore their *present*
branches are not selected for this frozen union, but the candidate does not
encode the conditional build dependency logic at all.  It only asserts their
current absence in a comment, which will neither register the currently
unselected contract nor prevent an incorrect future build integration from
silently omitting it.

## Frozen-evidence checks

- Scope maps this as common `RUST_TRANSLATE`; authoritative header closure has
  8,963 aarch64 and 2,967 x86_64 direct contexts (11,930 total), so this is a
  widespread build dependency, not an inert header.
- `PHASE0_IDENTITY.tsv` pins LLVM 19.1.7 clang at
  `/usr/lib/llvm-19/bin/clang`, `LLVM=/usr/lib/llvm-19/bin/`, `LLVM_IAS=1`,
  the stated Linux commit, and the two frozen configuration hashes.
- The complete frozen compiler-predicate inventory has 72 PROVEN entries (36
  per architecture), all bound to that identity and validated PASS.  It has no
  `source_locations` entry for `include/linux/compiler-version.h`; that header
  itself contains no `__has_*` predicate.  This does not reduce the three
  ordinary build-defined conditional dependencies above.
- `rewrite/SYMBOLS.tsv` still records the guard and all three `#ifdef` branches
  for both architectures as `PENDING_REVIEW`.  They must be closed with the
  actual force-include/dependency mapping before this task can be DONE.

No compiler, preprocessor, build, formatter, test, or runtime command was run
for this review.
