# S013611 applier resolution

## Disposition

**BLOCKED.** The candidate's invented `pub const __LINUX_COMPILER_VERSION_H:
()` has been removed. No Rust item replaces the C preprocessor macro because
the macro is not a C or ABI value; it is state in the forced-include
preprocessor protocol.

### P1 / R1 — compiler-version rebuild dependency

**Accepted; unresolved in the present frozen Rust build integration.** Pinned
`include/linux/compiler-version.h:8-14` deliberately contains the literal
`CONFIG_CC_VERSION_TEXT`. `scripts/basic/fixdep.c:73-85,189-204` scans such
text in prerequisites and records `include/config/CC_VERSION_TEXT`; pinned
`init/Kconfig:2-17` says Kconfig touches that dependency when the compiler
version changes. Both authoritative artifact inventories contain
`include/config/CC_VERSION_TEXT`, and both frozen configs bind it to Debian
clang 19.1.7, which matches `PHASE0_IDENTITY.tsv`.

The repository has no Rust build integration (`Cargo.toml` declares package
metadata only and there is no `build.rs`), and neither the candidate nor an
existing Rust build input represents this edge. A prose mention or a Rust
constant would not create a rebuild dependency. This task cannot create that
cross-file build mechanism under its one-file scope.

### P1 / R1 — forced inclusion and direct-include rejection

**Accepted; unresolved in the present frozen Rust build integration.** Pinned
`Makefile:584-592` adds `-include $(srctree)/include/linux/compiler-version.h`
to `USERINCLUDE`; representative frozen x86_64 and aarch64 `.cmd` records
contain the same forced include. Pinned header lines 3-6 reject an ordinary
later/direct inclusion after the forced inclusion. Rust module inclusion is not
equivalent: it neither force-includes the module for every translation unit nor
models the explicit rejection. No frozen Rust ABI or build record supplies an
alternative source-level mechanism.

### P1 / R1 — conditional generated-header dependencies

**Accepted; current branches are mechanically disabled, but their required
mapping is part of the missing build contract.** Pinned header lines 23-25,
32-34, and 42-44 conditionally include `generated/gcc-plugins.h`,
`generated/randstruct_hash.h`, and `generated/integer-wrap.h`. The frozen
configs select clang, `CONFIG_RANDSTRUCT_NONE=y`, and no UBSAN; the respective
Kbuild inclusion sites are `Makefile:1204,1206,1211`, and no corresponding
generated header exists in either frozen Kbuild tree. This proves the current
branches do not add an active dependency, not an alternative to the required
conditional build mapping.

### R2 — invented public Rust constant

**Accepted and fixed.** The unit-valued public constant added a Rust value
namespace API where upstream defines no object, linkage, layout, or value. It
has been removed; the remaining provenance-only file deliberately does not
claim to implement the preprocessor/build behavior.

## Pending records

All 18 S013611 `SYMBOLS.tsv` records remain `PENDING_REVIEW`: the forced
include guard/error protocol and all three conditional dependency branches lack
an exact Rust-side build mapping. They must not be closed while the task is
blocked.

## Required prerequisite

An approved frozen Rust build-integration mapping is required that, for both
architectures, force-applies the compiler-version protocol to every translated
consumer, rejects source-level/direct inclusion after that application, depends
on `include/config/CC_VERSION_TEXT`, and conditionally records the three
generated-header inputs using the pinned Kbuild conditions.

No compiler, preprocessor, build, formatter, test, or runtime command was run.
