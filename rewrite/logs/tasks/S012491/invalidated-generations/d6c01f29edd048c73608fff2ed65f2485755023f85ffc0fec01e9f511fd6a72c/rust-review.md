# Rust semantics review — S012491

Reviewer: rust_reviewer (independent)

## Scope examined

- Pinned source: `vendor/linux/include/acpi/platform/acgccex.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/acpi/platform/acgccex.rs`.
- Frozen queue/scope/symbol records, both frozen configurations, the generated
  header-closure evidence, and the direct ACPICA include context
  (`acpi.h`, `acenvex.h`, `acenv.h`, `aclinux.h`, and `aclinuxex.h`).

No compiler, formatter, test, linker, or Rust-analyzer diagnostic was invoked.

## Result

No Rust-semantics finding.

`acgccex.h` has no type, data, function, ABI, or FFI declaration.  Its sole
operative effect is textual C-preprocessor state: once the C include guard is
entered, conditionally remove a macro named `strchr` for the remainder of that
C translation unit.  The direct include chain is `acpi.h` -> `acenvex.h` ->
`acgccex.h` under `__GNUC__`; the header-closure inventory selects it for both
frozen architectures.

Rust has no C-style textual preprocessor macro namespace.  A Rust macro is in
the macro namespace and must be invoked with `!`; it cannot transparently
replace a bare item/function call such as C's `strchr(...)`, nor can a child
module remove a caller's macro binding.  Accordingly there is no sound or
semantically meaningful Rust item, `macro_rules!` definition, `#[cfg]`, FFI
binding, unsafe operation, lifetime, or module-side action corresponding to
the C `#ifdef strchr` / `#undef strchr` pair.  An empty Rust module is the
faithful representation of this C-only workaround; adding a `strchr` item or
macro would create behavior and namespace state absent from the source.

The pinned original header remains the header consumed by original Linux C
driver objects, so their C-preprocessor behavior is preserved independently
of this Rust module.  This header itself supplies no cross-language ABI or
layout contract.

The candidate's SPDX expression, Intel copyright notice, Linux source path,
revision (`425f94c2954b1fe80ebdbf9b29854e89750355df`), `common` architecture
membership, and task ID match the pinned source, `vendor/linux.SHA`, and the
S012491 queue/scope records.  It introduces no `unsafe`, exported symbol,
layout, panic path, runtime state, or configuration-dependent behavior.

## PENDING_REVIEW dispositions for applier

`rewrite/SYMBOLS.tsv` records, for both `aarch64` and `x86_64`:

- `ifndef@10` and operative macro `__ACGCCEX_H__`: C include-guard state only;
  no Rust item or ABI/lifetime obligation.
- `ifdef@20` (`strchr`): C macro-namespace test and conditional undefinition;
  no Rust counterpart for the reasons above.
- `endif@22` and `endif@24`: delimit the preceding C-preprocessor constructs;
  no Rust semantic artifact.

No S012491 record exists in `LIFETIMES.tsv`, `ABI.tsv`, `DRIVER_ABI.tsv`, or
`BLOCKERS.tsv`; that is consistent with the absence of values, storage,
pointers, FFI, or lifetime-bearing constructs in the upstream header.
