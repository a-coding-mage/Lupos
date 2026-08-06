# Parity review: S000805

Reviewed task `S000805` independently against pinned
`vendor/linux/arch/x86/include/uapi/asm/vmx.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen `x86_64` scope.

## Verdict

**REJECT: three parity findings require applier resolution.**

## Finding P1 — the upstream SPDX identifier changed

- **Pinned source:** `arch/x86/include/uapi/asm/vmx.h:1` declares
  `GPL-2.0 WITH Linux-syscall-note`.
- **Candidate:** `src/arch/x86/include/uapi/asm/vmx.rs:1` declares
  `GPL-2.0-only`.
- **Impact:** This changes the pinned UAPI header's SPDX identifier.  The
  rewrite protocol requires the upstream SPDX identifier to be retained.
- **Required resolution:** Retain the exact upstream SPDX expression in the
  Rust source header.

## Finding P2 — required upstream copyright notices were omitted

- **Pinned source:** `arch/x86/include/uapi/asm/vmx.h:3-23` retains the Intel
  copyright and the Qumranet / Avi Kivity / Yaniv Kamay notices.
- **Candidate:** has no corresponding copyright notices.
- **Impact:** The protocol requires relevant upstream copyright notices to be
  retained; these explicit notices are part of the authoritative header.
- **Required resolution:** Preserve the relevant upstream copyright notices in
  the translated file.

## Finding P3 — `VMX_EXIT_REASONS_SGX_ENCLAVE_MODE` changed signed integer type

- **Pinned source:** `arch/x86/include/uapi/asm/vmx.h:30` defines the unsuffixed
  hexadecimal integer constant `0x08000000`.  On the frozen x86_64 ABI this
  value is representable as a signed 32-bit `int`.
- **Candidate:** `src/arch/x86/include/uapi/asm/vmx.rs:8` exposes it as `u32`.
- **Impact:** This UAPI macro's signed integer category changed even though the
  value fits `i32`; direct signed uses no longer preserve the C expression's
  type.  By contrast, `0x80000000` at source line 29 correctly requires the
  unsigned 32-bit category.
- **Required resolution:** Represent `VMX_EXIT_REASONS_SGX_ENCLAVE_MODE` as
  `i32` while retaining its exact value.

## Exhaustive comparison performed

- The candidate provenance source path, revision, `x86_64` architecture, and
  task ID match the queue row and pinned revision.
- All 72 direct value macros are present with the exact names and numeric
  values.  The 70 decimal macros correctly use signed 32-bit values; the
  `0x80000000` failed-vmentry flag correctly uses `u32`.  The remaining flag is
  the signedness exception documented in P3.
- `VMX_EXIT_REASONS` preserves all 65 source `(exit reason, name)` pairs in
  source order, including the intentionally omitted `OTHER_SMI` and
  `SEAMCALL` entries.  Each mapped exit-reason value and string matches the
  pinned macro expansion.
- `VMX_EXIT_REASON_FLAGS` preserves its sole
  `(VMX_EXIT_REASONS_FAILED_VMENTRY, "FAILED_VMENTRY")` entry with a 32-bit
  unsigned flag value.  The three VMX abort-code values also match.
- The header has no selected configuration branches, declarations, layouts,
  functions, storage definitions, or exported C symbols beyond these macros;
  no branding allowlist entry applies.  The include guard has no Rust runtime
  declaration analogue.

No source, manifest, or non-review evidence file was modified by this
reviewer. No build, compiler, formatter, test, linker, or runtime command was
run.
