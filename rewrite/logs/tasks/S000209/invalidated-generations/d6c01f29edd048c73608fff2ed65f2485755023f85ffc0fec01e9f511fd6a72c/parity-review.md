# S000209 parity review (slot 1)

Reviewed independently against pinned Linux `425f94c2954b1fe80ebdbf9b29854e89750355df`:
`arch/arm64/include/uapi/asm/auxvec.h`, the frozen AArch64 task/symbol scope,
and direct ARM64/generic ELF auxiliary-vector consumers.  No compiler,
formatter, linker, test, runtime tool, diagnostic, historical Rust source, or
implementation rationale was used.

## Findings

1. **Major — source integer expression types and promotions changed.**
   Linux defines `AT_SYSINFO_EHDR`, `AT_MINSIGSTKSZ`, and
   `AT_VECTOR_SIZE_ARCH` as unsuffixed decimal integer literals.  Each macro
   therefore has C `int` expression type before its consumer-specific
   conversion.  The candidate exports all three as `u64`.  This changes the
   definition's direct type and, in particular, changes the arithmetic domain
   of `AT_VECTOR_SIZE_ARCH`: `include/linux/mm_types.h` uses it in
   `2 * (AT_VECTOR_SIZE_ARCH + AT_VECTOR_SIZE_BASE + 1)`, which is C integer
   arithmetic.  In `fs/binfmt_elf.c`, `NEW_AUX_ENT` subsequently converts the
   two ARM64 `ARCH_DLINFO` tag expressions to `elf_addr_t`; that later
   conversion is not a basis for widening the source macros.  Preserve the
   C-int representation at the declaration boundary and make any required
   consumer conversion explicit at the corresponding use.

2. **Major — the upstream GPL notice was dropped.**
   In addition to the retained SPDX identifier and `Copyright (C) 2012 ARM
   Ltd.`, the complete pinned UAPI header contains the GPLv2 redistribution,
   warranty-disclaimer, and license-copy notice at lines 3–16.  The candidate
   retains only the copyright line.  This does not preserve the relevant
   upstream copyright/license notice required for the path-preserving source
   translation; restore that notice verbatim as comments.

## Verified parity

- `AT_SYSINFO_EHDR == 33`, `AT_MINSIGSTKSZ == 51`, and
  `AT_VECTOR_SIZE_ARCH == 2` have the exact pinned numeric values.
- The source has no configuration-selected branch: its only conditional is
  the standard `__ASM_AUXVEC_H` include guard.  Rust module idempotence is the
  appropriate language-level counterpart; no runtime branch was omitted.
- ARM64 `ARCH_DLINFO` in `arch/arm64/include/asm/elf.h` always emits two auxv
  entries: the vDSO tag and either `AT_MINSIGSTKSZ` or `AT_IGNORE`.  The pinned
  `AT_VECTOR_SIZE_ARCH` value of 2 consequently agrees with its allocation
  use in `include/linux/mm_types.h` and the generic `fs/binfmt_elf.c` contract.
- The candidate provenance names the exact Linux path/revision, AArch64 scope,
  and assigned task ID; its SPDX expression matches the source.

## Disposition

**Changes required.** Both findings need source correction before final
application can establish zero-difference parity.
