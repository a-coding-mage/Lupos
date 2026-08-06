# Parity review — S000772 (slot 1)

## Verdict

PASS — no source-parity finding.

## Evidence reviewed

- Pinned source: `vendor/linux/arch/x86/include/uapi/asm/debugreg.h`, complete
  101-line file, at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/arch/x86/include/uapi/asm/debugreg.rs`.
- Frozen task scope: x86_64 only (`S000772`); the Phase 0 x86_64 target is
  `x86_64-linux-gnu`.
- Relevant consumers including the `unsigned long` debug-register paths:
  `arch/x86/kernel/hw_breakpoint.c`, `arch/x86/kernel/ptrace.c`,
  `arch/x86/kernel/kgdb.c`, `arch/x86/kernel/traps.c`,
  `arch/x86/mm/kmmio.c`, and `arch/x86/include/asm/traps.h`.

## Exhaustive comparison

- All 32 active x86_64 macro values are present with their upstream names and
  values. `DR_TRAP_BITS` retains the upstream OR composition.
- The 30 ordinary unsuffixed integer-literal macros are represented as `i32`,
  matching their C `int` literal category. `DR6_RESERVED` is `u32`, matching
  the unsuffixed hexadecimal literal's `unsigned int` category on x86_64;
  `DR_CONTROL_RESERVED` is `core::ffi::c_ulong`, matching the selected
  x86_64 `UL` literal and its 64-bit mask `0xFFFFFFFF0000FC00UL`.
- The frozen task excludes i386. The candidate therefore correctly selects
  the source `#else` value for `DR_CONTROL_RESERVED`, not the i386 `0xFC00`
  branch. The C include guard has no Rust data or ABI counterpart.
- Consumer use is consistent with the preserved values and categories:
  unsigned-long DR6/DR7 operations receive the normal C conversions from
  these integer macro values; no value, mask, shift, or reserved-bit polarity
  is altered by the candidate declaration.
- This UAPI header declares no objects, functions, structures, layouts,
  exported linkage, or calling convention. Public constants preserve the
  header's named compile-time interface without inventing an ABI symbol.
- SPDX is exactly `GPL-2.0 WITH Linux-syscall-note`; provenance source, Linux
  revision, architecture, and task ID match the frozen task. No branding delta,
  placeholder, test code, or out-of-scope i386 declaration is present.
