# S000779 applier resolution

Applier: `gpt-5.6-terra`, high effort

## Evidence reopened

I reopened the complete pinned `arch/x86/include/uapi/asm/ldt.h`, its direct
x86 consumers `arch/x86/include/asm/desc.h`, `arch/x86/kernel/tls.c`,
`arch/x86/kernel/ldt.c`, and `arch/x86/kernel/ptrace.c`, the frozen
x86_64 configuration, and the Phase 0 identity.  The identity pins Debian
clang 19.1.7 at `/usr/lib/llvm-19/bin/clang`, target
`x86_64-linux-gnu`, and LLVM IAS.  The recorded original `ldt.o` command and
its hash are present in `rewrite/kbuild/x86_64/arch/x86/kernel/.ldt.o.cmd` and
`rewrite/metadata/x86_64/all_artifacts.tsv:700` respectively.

The frozen configuration has `CONFIG_DEBUG_INFO_NONE=y`.  The hash-matched
`rewrite/kbuild/x86_64/arch/x86/kernel/ldt.o` has neither DWARF nor BTF type
metadata, and `rewrite/kbuild/x86_64/vmlinux` has no BTF or DWARF section.
Consequently, the retained Phase 0 artifacts prove the compiler command and
source provenance, but do not record the compiler's `struct user_desc`
bit-field layout.

## Finding dispositions

| Finding | Disposition |
| --- | --- |
| P1 / Rust HIGH: prove the locked LLVM x86_64 bit-field ABI before accepting the trailing `u32` and bit positions | **BLOCKED.** `ldt.h:24-40` uses C `unsigned int` bit-fields whose allocation unit, ordering, offsets, size, and alignment are implementation-defined. The pinned source, frozen configuration, compiler-predicate inventory, and retained object metadata do not establish them. The candidate's `u32` layout and masks therefore remain an unapproved guess. |
| Rust MEDIUM: preserve the `lm == 0` rule for a `user_desc` originating from a 32-bit program | **Accepted as a required follow-on constraint, not closed.** `ldt.h:32-39` requires every such consumer to ignore the stored bit and act as though `lm == 0`; `desc.h:353-362` likewise ignores `lm` in `LDT_empty`, and `desc.h:35-41` refuses to consume it in `fill_ldt`. The candidate's raw `lm()` accessor alone does not establish this provenance-sensitive consumer contract. Do not sanitize `bits`, since the physical word and its unassigned bits are copied as UAPI data. Once the ABI prerequisite is supplied, the source must distinguish raw/native `lm` access from compat-originated consumption that returns zero (or bind every compat consumer to an equivalent zero rule), followed by fresh independent review. |
| Parity P1: complete S000779 task records | **Not permitted while blocked.** The records cannot truthfully be changed from `PENDING_REVIEW` until the ABI layout prerequisite exists. |

All five task artifacts are present and were considered: implementation record,
candidate diff, parity review, Rust review, and this resolution.  No source
edit is accepted because it would encode the same unproven bit-field ABI.

## Single prerequisite to resume

Phase 0 must generate, independently validate, and bind to the existing
`PHASE0_IDENTITY` a retained record-layout artifact for the *original*
`struct user_desc`, using the exact frozen clang-19 x86_64 `ldt.o` command
context.  It must state: `unsigned int` width/alignment; member offsets 0, 4,
and 8; bit-field allocation-unit offset/width; all seven declared bit offsets
and widths (including `lm`); and whole-struct size/alignment.  The artifact's
command, raw output, compiler/input hashes, and validation evidence must be
auditable.  This is a Phase 0 action, not a Phase 1 compile or probe.

No compiler, formatter, build, test, emulator, debugger, benchmark, or
historical Rust source was used.
