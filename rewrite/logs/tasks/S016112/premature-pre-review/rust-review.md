# Rust semantic review — S016112, attempt 2, slot 2

Scope reviewed independently: `vendor/linux/include/uapi/linux/elf-em.h` and
the candidate snapshot for `src/include/uapi/linux/elf-em.rs`. No compiler,
formatter, test, runtime command, rust-analyzer diagnostic, historical Rust
source, implementation rationale, or other review report was used.

## Result: APPROVE

The C header contains only an include guard and object-like integer-literal
macros. The candidate exposes one immutable Rust `i32` constant for every
`EM_*` macro, preserving the identifier, duplicate value
(`EM_MIPS_RS3_LE`/`EM_MIPS_RS4_BE`), and each decimal or hexadecimal value.
All literals are representable by the 32-bit signed `int` type used by both
approved targets, so the selected `i32` value domain agrees with the C
integer-constant expressions. There are no expressions, casts, shifts,
pointer operations, aliases, ownership transitions, callbacks, allocation
paths, layout-bearing declarations, extern linkage, or unsafe blocks in the
upstream header or candidate.

The C include guard governs repeated preprocessing of the C header. It does
not create a runtime object, ABI field, or Rust linkage requirement, and the
destination is a single Rust module with equivalent single-definition module
semantics. The UAPI values remain public constants; no C ABI surface is
introduced or altered by this header-only translation.

Evidence: upstream lines 2–71 and candidate snapshot SHA-256
`7420c60cdafb8a4b6f5bbea52fdc439224d34adfef2ae0b218215460396e0918`.
