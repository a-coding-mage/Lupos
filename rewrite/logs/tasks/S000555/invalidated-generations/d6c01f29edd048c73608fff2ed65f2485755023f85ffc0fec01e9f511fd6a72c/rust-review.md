# Rust review — S000555

Reviewer: `rust_reviewer` (`gpt-5.6-terra`, high)  
Scope: source-only review of `src/arch/x86/include/asm/inat_types.rs` against
`vendor/linux/arch/x86/include/asm/inat_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding R1 — upstream SPDX identifier was narrowed

`vendor/linux/arch/x86/include/asm/inat_types.h:1` identifies the source as
`GPL-2.0-or-later`, while `src/arch/x86/include/asm/inat_types.rs:1` says
`GPL-2.0-only`.  The task's frozen source is the former; narrowing it does not
retain the upstream SPDX identifier.  The applier must reconcile this with the
required provenance convention using the pinned-source authority before DONE.

## Rust ownership, layout, and FFI assessment

No other Rust finding.  The three selected C aliases are represented exactly
for the frozen x86_64 ABI:

- `unsigned int` (`inat_types.h:11`) is `u32`;
- `unsigned char` (`inat_types.h:12`) is `u8`;
- `signed int` (`inat_types.h:13`) is `i32`.

`rewrite/configs/x86_64/frozen.config` selects the x86_64/64-bit target, and
the recorded original Kbuild command in `rewrite/FILE_MAP.tsv` for this header
uses `--target=x86_64-linux-gnu` and `-m64`.  These scalar aliases therefore
preserve width, signedness, alignment, and by-value C-ABI representation for
the selected consumers (including the declarations in
`arch/x86/include/asm/inat.h:106-118` and fields in
`arch/x86/include/asm/insn.h:18-45,97,103-105`).  Type aliases introduce no
Rust drop, ownership, aliasing, or `Send`/`Sync` state; no `repr` annotation or
`unsafe` block is required or present.

No compiler, formatter, build, test, debugger, or rust-analyzer diagnostic was
used.
