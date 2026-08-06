# Parity review — S000803 (attempt 2, slot 1)

Scope reviewed: `vendor/linux/arch/x86/include/uapi/asm/unistd.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, current
`src/arch/x86/include/uapi/asm/unistd.rs`, and the frozen x86_64 scope,
symbol, header-closure, configuration, and queue records.  This was a
source-only review; no compiler, formatter, test, or runtime tool was used.

## Finding P1 — SPDX identifier is not retained

`arch/x86/include/uapi/asm/unistd.h:1` declares
`GPL-2.0 WITH Linux-syscall-note`.  Candidate line 1 instead declares
`GPL-2.0-only`.  This alters the pinned UAPI file's SPDX licensing exception
and violates the required retention of the upstream SPDX identifier.

Required resolution: use the exact upstream SPDX identifier in the candidate.

## Checked parity points

- `__X32_SYSCALL_BIT` is `0x40000000`; the candidate preserves the value and
  represents its C `int` type as `i32`, matching the upstream comment's
  required signed-int flag behavior.
- The candidate's source path, Linux revision, `x86_64` architecture, and task
  ID provenance are exact matches for frozen task `S000803`.
- The frozen header-closure evidence selects this header through kernel
  consumers whose recorded command defines `__KERNEL__`.  Consequently the
  `#ifndef __KERNEL__` include dispatch to `unistd_32.h`, generated
  `unistd_x32.h`, or generated `unistd_64.h` is excluded for the selected
  kernel context.  No unselected user-UAPI branch was invented in Rust.
- The frozen x86_64 configuration enables `CONFIG_IA32_EMULATION` but has no
  `CONFIG_X86_X32_ABI` setting; this does not make the excluded C
  `__ILP32__` UAPI dispatch an active selected branch.

Conclusion: reject the candidate until P1 is resolved.  No other source-level
parity difference was found in the reviewed scope.
