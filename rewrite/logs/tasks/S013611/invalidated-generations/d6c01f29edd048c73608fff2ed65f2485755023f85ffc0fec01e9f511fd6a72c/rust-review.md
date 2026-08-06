# Rust review — S013611

Status: **REJECT** — one blocker and one high-severity source-representation defect.

## R1 — blocker: the candidate only describes, rather than preserves, the header's build-time contract

`include/linux/compiler-version.h` is forcibly included in every recorded C/assembly compile command by `vendor/linux/Makefile:586-592`.  Its literal `CONFIG_CC_VERSION_TEXT` is deliberately consumed by `fixdep` (`vendor/linux/init/Kconfig:2-17`) so a compiler-version change forces a complete rebuild.  The candidate merely comments on that contract; it provides no mechanism or bound record through which the Rust build can depend on the frozen compiler-version value.  This is material, not a host inference: both frozen configurations contain `CONFIG_CC_VERSION_TEXT="Debian clang version 19.1.7 (3+b1)"`, and `rewrite/PHASE0_IDENTITY.tsv` binds that same LLVM 19.1.7 compiler at `/usr/lib/llvm-19/bin/clang` for `x86_64-linux-gnu` and `aarch64-linux-gnu`.

The candidate also has no mapping for the selected `#ifdef __LINUX_COMPILER_VERSION_H` / `#error` behavior and three conditional dependency branches, all of which remain `PENDING_REVIEW` in the S013611 records in `rewrite/SYMBOLS.tsv`.  A prose assertion is not an implementation of those selected preprocessor/build effects.  The applier must establish an auditable Rust-side build dependency and an exact mapping for the preprocessor-only include-guard/error behavior; if that cannot be done within the frozen task scope, the task must be `BLOCKED`, not closed with a documentation-only file.

## R2 — high: `pub const __LINUX_COMPILER_VERSION_H: ()` is an invented Rust API, not the C macro

At `src/include/linux/compiler-version.rs:17`, the candidate exports a public zero-sized Rust constant in place of the C preprocessor macro.  The upstream `#define __LINUX_COMPILER_VERSION_H` has no C object, linkage, value, or public API; it only controls the immediately preceding forced-include guard.  No Rust consumer uses this constant.  It neither reproduces the diagnostic on a second/manual C inclusion nor contributes to Kbuild/fixdep dependency calculation, while it introduces a new Rust value-namespace item.  This violates the required one-to-one semantic mapping and the prohibition on constants standing in for an operative source file.  Remove it unless a real, evidence-backed Rust consumer contract requires an item of this exact shape (none was found).

## Frozen-branch evidence

The branch conditions themselves are accurately disabled for the frozen union, but that does not resolve R1/R2:

- Both frozen configs set `CONFIG_CC_IS_CLANG=y`, `CONFIG_GCC_VERSION=0`, `CONFIG_RANDSTRUCT_NONE=y`, and `# CONFIG_UBSAN is not set`.
- `scripts/Makefile.gcc-plugins` adds `-DGCC_PLUGINS` only when `CONFIG_GCC_PLUGINS` includes that file; its Kconfig depends on `CC_IS_GCC`, which is false here.
- `scripts/Makefile.randstruct` adds `-DRANDSTRUCT` only when `CONFIG_RANDSTRUCT`; the frozen `RANDSTRUCT_NONE` selection makes it false.
- `scripts/Makefile.ubsan` adds `-DINTEGER_WRAP` only for `CONFIG_UBSAN_INTEGER_WRAP`; it is unavailable with UBSAN disabled.
- Neither architecture's archived `generated-headers-include-generated.tar` contains `gcc-plugins.h`, `randstruct_hash.h`, or `integer-wrap.h`, consistent with the disabled branches.

`rewrite/compiler-predicates/COMPILER_PREDICATES.tsv`/`VALIDATION.tsv` contain 72 independently replayed compiler-feature predicates and do not contain a predicate for this header; they cannot substitute for the required compiler-version/fixdep mapping.

## Rust-safety checks

No `unsafe`, FFI layout, ownership, aliasing, panic, allocation, arithmetic, or `Drop` issue exists in the candidate.  The rejection is solely its missing build/preprocessor semantics and invented public constant.
