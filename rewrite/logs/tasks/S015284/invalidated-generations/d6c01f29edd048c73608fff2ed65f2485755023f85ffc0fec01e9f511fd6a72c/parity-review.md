# Parity review — S015284

Reviewed `vendor/linux/include/linux/uts.h` in full against
`src/include/linux/uts.rs`, the frozen x86_64 and AArch64 configurations, the
task scope/symbol manifests, and the direct pinned consumer
`vendor/linux/init/version-timestamp.c`.  This was a source-only review; no
compiler, formatter, linker, test, or runtime tooling was invoked.

## Finding P1 — C string-literal initializer semantics are not preserved

`uts.h` defines `UTS_SYSNAME`, `UTS_NODENAME`, and `UTS_DOMAINNAME` as
object-like macros whose selected expansions are C string literals.  A C string
literal includes its trailing NUL and can be used directly as an aggregate
initializer.  The direct selected consumer at
`vendor/linux/init/version-timestamp.c:13-18` initializes the `char` array
members of `struct new_utsname` with these macros.

The candidate instead exports each value as `&str`.  A Rust `&str` is a
fat-pointer/length slice, does not include the trailing NUL, and is not an
equivalent C character-array initializer.  The header supplies no
NUL-terminated byte representation for the translated `init_uts_ns.name`
initialization, so the source contract of all three macros is lost even though
their visible text matches.

The applier must represent the selected macro values in a form that preserves
the C string-literal byte sequence (including the terminating NUL) and permits
the corresponding fixed-size UTS-name field initialization without pointer or
length substitution.  Preserve the selected values: `"Linux"`, `"(none)"`,
and `"(none)"` respectively.

## Checked parity facts

- Both frozen configurations set `CONFIG_DEFAULT_HOSTNAME="(none)"`; the
  selected `UTS_NODENAME` text is therefore correctly identified as `"(none)"`.
- The approved x86_64/AArch64 source and recorded compile-command context have
  no `UTS_*` predefinition; the active `#ifndef` defaults are the three values
  above.  No out-of-scope architecture override was treated as selected
  behavior.
- No structs, exported functions, linkage declarations, or layout-bearing
  objects occur in this header.
- `UTS_SYSNAME` remains `Linux`, which is required because the branding
  allowlist contains no permitted name delta.
- The candidate provenance identifies the correct Linux path, revision, common
  architecture scope, and task ID.

Result: changes required before parity acceptance.
