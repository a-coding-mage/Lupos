# Parity review — S000472 (slot 1)

Scope reviewed: `vendor/linux/arch/x86/include/asm/audit.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 configuration,
the S000472 `SYMBOLS.tsv`/`ABI.tsv` records, and required local audit context
(`arch/x86/ia32/audit.c` and `arch/x86/kernel/audit_64.c`).  No compiler,
formatter, test, or runtime tool was used.

Verdict: **reject pending correction**.

## Findings

1. **P1 — all five C array declarations are narrowed to scalar foreign
   statics.**  The pinned header declares `ia32_dir_class`,
   `ia32_write_class`, `ia32_read_class`, `ia32_chattr_class`, and
   `ia32_signal_class` as `extern unsigned ...[]` at lines 7–11.  The candidate
   instead declares each as `pub static mut ...: u32`.  That retains the symbol
   spelling and the element width, but changes the public declaration from an
   incomplete mutable array to a scalar object: a Rust user can perform a
   scalar value access, while the C declaration denotes an array and decays to
   its first-element address at use sites.  This is material audit-table ABI
   surface, not merely documentation.  Required context confirms that
   `arch/x86/ia32/audit.c` defines all five as mutable `unsigned []` tables and
   `arch/x86/kernel/audit_64.c:66–70` passes their decayed addresses to
   `audit_register_class` under frozen `CONFIG_IA32_EMULATION=y`.  Preserve an
   array-form foreign interface (with bounds derived from the same frozen
   generated syscall input where Rust requires a bound), rather than replacing
   the declarations with `u32` scalars.

2. **P2 — SPDX identifier was changed without an allowlist entry.**  The
   pinned header begins `/* SPDX-License-Identifier: GPL-2.0 */`, while the
   candidate begins `// SPDX-License-Identifier: GPL-2.0-only`.  These are
   different SPDX identifiers and the branding allowlist has no such delta.
   Retain the upstream identifier exactly.

## Checked parity points

`ia32_classify_syscall(unsigned int) -> int` is otherwise represented with the
matching x86_64 widths as `unsafe extern "C" fn(u32) -> i32`, and its symbol
spelling is unchanged.  The candidate retains all five expected table symbol
names and declares them mutable.  The source header has only its include guard;
it contains no configuration conditional.  The frozen configuration selects
the x86_64 audit context (`CONFIG_AUDIT=y`, `CONFIG_AUDITSYSCALL=y`, and
`CONFIG_IA32_EMULATION=y`), so no source-level conditional in this header may
drop these declarations.  The Linux path, revision, architecture, and task ID
provenance fields match the S000472 queue row.
