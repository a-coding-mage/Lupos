# Parity review — S013482

Reviewed `vendor/linux/include/linux/audit_arch.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/audit_arch.rs`, for the frozen `x86_64` and `aarch64`
configuration union.  No compiler, formatter, linker, test, or diagnostic was
run.

## Findings

1. **P1 — the five `extern unsigned int ...[]` declarations lost their
   incomplete-array type.**  Linux lines 27–31 declare
   `compat_write_class`, `compat_read_class`, `compat_dir_class`,
   `compat_chattr_class`, and `compat_signal_class` as external arrays of
   `unsigned int` with an incomplete bound.  The candidate declares each as a
   scalar `u32` static instead.  This preserves the symbol address only when a
   caller manually takes the scalar's address, but it does not preserve the
   declaration's array type or normal C array-to-pointer use.  The selected
   aarch64 generic implementation confirms this is operative: the five
   symbols are defined as arrays in `lib/compat_audit.c:7–30`, and
   `lib/audit.c:75–79` passes each array's decayed address to
   `audit_register_class`.  Bind each external to a zero-length/opaque Rust
   array representation (or an equivalent binding that exposes a pointer to
   an incomplete `u32` array) rather than a scalar first element.

2. **P2 — source license/provenance was changed and the upstream copyright
   notice was dropped.**  The pinned header is
   `SPDX-License-Identifier: GPL-2.0-or-later` and carries the Red Hat 2021
   copyright and Richard Guy Briggs attribution in lines 2–7.  The candidate
   instead states `GPL-2.0-only` and omits those notices.  This does not retain
   the original SPDX identifier or relevant upstream copyright/provenance.

## Confirmed parity

- `auditsc_class_t` has all eight enumerators in the original order and values:
  `AUDITSC_NATIVE = 0`, then `COMPAT`, `OPEN`, `OPENAT`, `SOCKETCALL`,
  `EXECVE`, `OPENAT2`, and `NVALS = 7`.  `#[repr(C)]` is appropriate for the
  C enum representation.
- `audit_classify_compat_syscall` retains its external C symbol name and C ABI,
  with `int` mapped to `i32` and `unsigned int` mapped to `u32`.  The source
  function in `lib/compat_audit.c:32–56` returns the integer enum constants;
  no function body belongs in this declaration header.
- The header's include guard has no run-time or exported-object analogue to
  preserve in the path-mirrored Rust module.  Its enum and all six external
  declarations are unconditional for both selected architectures.
- Frozen configuration evidence selects the header for both architectures;
  only aarch64 selects `CONFIG_AUDIT_GENERIC` and
  `CONFIG_AUDIT_COMPAT_GENERIC`, which is why the generic array consumer and
  definitions are active there.  x86_64 uses its architecture-specific IA32
  audit arrays instead.  Neither fact removes the header's five common
  external declarations from the required interface.

## Verdict

**REJECT pending P1 and P2.**
