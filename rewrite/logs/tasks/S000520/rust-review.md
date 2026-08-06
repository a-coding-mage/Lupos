# Rust review — S000520 (slot 2)

Scope checked source-only: `S000520` is leased by `P01` and is `REVIEWING` for
`arch/x86/include/asm/emulate_prefix.h` to
`src/arch/x86/include/asm/emulate_prefix.rs`, architecture `x86_64`.  The
candidate provenance revision equals `vendor/linux.SHA`
(`425f94c2954b1fe80ebdbf9b29854e89750355df`).  No compiler, formatter,
analyzer, build, test, or runtime tool was used.

## Findings

1. **High — the Rust items do not preserve the source macros' token-expansion
   interface.**  Linux lines 11--12 define `__XEN_EMULATE_PREFIX` and
   `__KVM_EMULATE_PREFIX` as untyped comma-separated preprocessing tokens, not
   objects: `0x0f,0x0b,...`.  Their expansion is context-sensitive.  In
   `arch/x86/lib/insn.c:82-83` and `arch/x86/kvm/x86.c:8024`, each expands into
   five initializer elements; in `arch/x86/include/asm/xen/interface.h:387`,
   `__XEN_EMULATE_PREFIX` expands within `__ASM_FORM(.byte ... ;)` into
   assembler operands.  The candidate instead provides two `pub const [u8; 5]`
   values (lines 13 and 16).  A Rust constant is one typed array expression,
   evaluated/copied by value at each use, and cannot expand into five array
   elements or into the `.byte` operand token sequence.  It therefore changes
   both macro form and the contexts in which the selected API can be used.
   `CONFIG_XEN` being unset in the frozen x86 config does not make this safe:
   the selected header has no configuration conditional around either macro,
   and `SYMBOLS.tsv` lists both as operative selected macros.  The applier must
   preserve the required macro/assembler and initializer expansion semantics,
   rather than accepting typed array constants as a replacement.

2. **Medium — SPDX provenance was altered.**  The complete pinned header's
   line 1 is `/* SPDX-License-Identifier: GPL-2.0 */`; candidate line 1 says
   `// SPDX-License-Identifier: GPL-2.0-only`.  These are different SPDX
   identifiers.  The rewrite rules require retaining the upstream SPDX
   identifier, so the candidate must retain the pinned header's identifier.

## Manual Rust-safety assessment

The two arrays contain values representable by `u8` and introduce no unsafe,
layout, ownership, aliasing, or drop issue by themselves.  That does not cure
finding 1: the defect is the loss of C preprocessing/assembler expansion
semantics before element typing is even determined by a consumer.
