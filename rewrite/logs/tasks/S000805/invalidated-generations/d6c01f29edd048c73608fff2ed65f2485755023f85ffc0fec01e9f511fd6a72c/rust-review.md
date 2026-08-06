# Rust review — S000805

Reviewed candidate: `src/arch/x86/include/uapi/asm/vmx.rs`  
Pinned source: `vendor/linux/arch/x86/include/uapi/asm/vmx.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` (x86_64)

Result: **changes required**.

## Findings

1. **HIGH — `VMX_EXIT_REASONS` and `VMX_EXIT_REASON_FLAGS` lost their
   context-dependent C initializer-list/FFI semantics.**

   Upstream lines 101-166 and 168-169 define macro *fragments*, not standalone
   objects.  Each expansion is typed by its consumer.  The selected kernel
   consumer `arch/x86/kvm/trace.h:385-389` expands them through
   `__print_symbolic` / `__print_flags_u64`; on frozen x86_64 those macros
   declare `static const struct trace_print_flags[]` (`include/trace/stages/
   stage3_trace_output.h:69-83`).  Its fields are `unsigned long mask` and
   `const char *name` (`include/linux/tracepoint-defs.h:16-19`).  The source
   header is also used by perf's `define_exit_reasons_table` in
   `tools/perf/util/kvm-stat-arch/kvm-stat-x86.c:12`, which declares
   `struct exit_reasons_table[]` with `unsigned long exit_code` and
   `const char *reason` (`tools/perf/util/kvm-stat.h:71-74,129-132`).

   In contrast, Rust lines 78-148 publish fixed `[(i32, &str); 65]` and
   `[(u32, &str); 1]` constants.  Rust tuples have no C representation,
   `&str` is a fat pointer rather than the required NUL-terminated `const
   char *`, and neither integer is the x86_64 consumer's `unsigned long`.
   The `const` arrays also cannot be substituted for the source's caller-local
   initializer fragments.  This removes the required consumer-specific table
   construction and supplies an incompatible FFI/layout surface.  Replace
   this with a representation and consumer adaptation that preserves each
   selected caller's exact `unsigned long`/C-string table contract; record the
   resulting ABI and static-lifetime decision before closing the task.

2. **MEDIUM — `VMX_EXIT_REASONS_SGX_ENCLAVE_MODE` has the wrong C integer
   category.**

   The unsuffixed hexadecimal literal `0x08000000` at `vmx.h:30` fits in a
   signed 32-bit `int`, so under the frozen x86_64 C model it is `int`/`i32`.
   Rust line 8 instead declares `u32`.  This matters because the header
   exposes macro constant-expression typing: the actual VMCS field is `u32`
   and C applies its usual conversions at the bitwise operation in
   `arch/x86/kvm/vmx/nested.c:4741`, rather than assigning the macro an
   unsigned type at definition.  Preserve the literal's signed category or
   provide a documented, context-preserving conversion at the actual u32 use.
   (The adjacent `0x80000000` at `vmx.h:29` does not fit `int` and is correctly
   represented as `u32`.)

3. **LOW — upstream UAPI SPDX and copyright notices were not retained.**

   The source begins `SPDX-License-Identifier: GPL-2.0 WITH
   Linux-syscall-note` and carries the Intel/Qumranet copyright block
   (`vmx.h:1-24`).  The candidate replaces this with `GPL-2.0-only` at Rust
   line 1 and omits the notices.  Restore the exact upstream SPDX identifier
   and relevant copyright notice; immutable task provenance belongs alongside
   it, not in place of it.

## Checked successfully

- All 72 direct object-like macro names are present.  The 65
  `VMX_EXIT_REASONS` value/string pairs retain upstream order, values, and
  spelling; `EXIT_REASON_OTHER_SMI` remains defined but is intentionally not
  a member of that upstream table.
- The direct decimal exit-reason and abort-code literals all fit the frozen
  C `int` range and are represented as `i32`.  There are no source shift
  expressions, structs, unions, functions, mutable storage, ownership
  transfers, synchronization operations, or `unsafe` blocks in this header.
- No build, format, test, compiler, linker, or runtime command was run, and
  this reviewer edited only this assigned review artifact.
