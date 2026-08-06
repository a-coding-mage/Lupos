# S012500 parity review (slot 1)

Reviewed `vendor/linux/include/asm-generic/audit_dir_write.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/asm-generic/audit_dir_write.rs` and the frozen selected call
contexts:

- `arch/x86/kernel/audit_64.c:8` (x86_64 native);
- `arch/x86/ia32/audit.c:6` (x86_64 IA32);
- `lib/audit.c:7` (AArch64 native); and
- `lib/compat_audit.c:7` (AArch64 AArch32 compatibility).

The four exported callback macros reproduce the selected conditional members
and their source order: 15 entries for each x86_64/IA32/AArch32-compat
context and 7 entries for AArch64 native.  The numeric values agree with the
pinned x86 syscall tables, the AArch64 generic syscall definitions, and the
AArch32 compatibility syscall table.  They remain reinvocable, and leaving
the `~0U` terminal value to each receiving array preserves the upstream
inclusion-fragment/sentinel boundary, including the nested use from
`include/asm-generic/audit_write.h`.

## Finding P1 — SPDX provenance identifier changed

`src/include/asm-generic/audit_dir_write.rs:1` says
`SPDX-License-Identifier: GPL-2.0-only`, whereas the pinned source at
`include/asm-generic/audit_dir_write.h:1` says
`SPDX-License-Identifier: GPL-2.0`.  No branding allowlist entry authorizes
this identifier change.  Restore the exact upstream SPDX identifier in the
candidate before closure; the source-header provenance requirement applies to
this header even though its executable content is represented by Rust macros.

No other parity findings.
