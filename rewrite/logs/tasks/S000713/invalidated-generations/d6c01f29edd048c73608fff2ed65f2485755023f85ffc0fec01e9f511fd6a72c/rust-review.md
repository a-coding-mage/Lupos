# Rust review — S000713

Reviewed as the independent Rust/FFI reviewer for P02, attempt 1. This was a
source-only review: no compiler, formatter, rust-analyzer, build, test, or
runtime tool was invoked. The assigned row was `REVIEWING`, mapped
`arch/x86/include/asm/syscalls.h` to `src/arch/x86/include/asm/syscalls.rs`,
was x86_64-only, and carried high risk.

## Result

No Rust-semantics, ABI, ownership, unsafe, or provenance findings.

## Evidence and coverage

- The complete upstream header has exactly one operative declaration:
  `long ksys_ioperm(unsigned long from, unsigned long num, int turn_on);`
  (`vendor/linux/arch/x86/include/asm/syscalls.h:8-15`). The candidate has
  exactly one public C-ABI declaration with the same symbol and argument
  order (`src/arch/x86/include/asm/syscalls.rs:8-10`).
- The candidate's `c_ulong, c_ulong, c_int -> c_long` preserves the C
  declaration's two `unsigned long` parameters, signed `int` parameter, and
  signed `long` result. In particular, `turn_on` is not `unsigned int`, so a
  `c_uint` substitution would be incorrect. The frozen x86_64 configuration
  selects `CONFIG_64BIT=y` and `CONFIG_X86_64=y`
  (`rewrite/configs/x86_64/frozen.config:313-315`), while the selected x86
  UAPI width header sets `__BITS_PER_LONG` to 64 for `__x86_64__` non-ILP32
  builds (`vendor/linux/arch/x86/include/uapi/asm/bitsperlong.h:5-10`). The
  Phase 0 identity fixes the target as `x86_64-linux-gnu`
  (`rewrite/PHASE0_IDENTITY.tsv:41`). Thus the target C ABI represented by
  `core::ffi::{c_ulong, c_long, c_int}` is the required LP64 ABI.
- `unsafe extern "C"` correctly makes an FFI call unsafe, rather than
  manufacturing a safe Rust contract for a kernel function whose implementation
  mutates `current->thread` state and depends on capability/security state
  (`vendor/linux/arch/x86/kernel/ioport.c:71-156`). This header declaration has
  no pointer, reference, aggregate, variadic, callback, ownership, or
  allocation-transfer parameter, so it introduces no Rust pointer provenance,
  aliasing, `Send`/`Sync`, pinning, or `Drop` obligation.
- The declaration has no C `static`, calling-convention annotation, visibility
  restriction, layout-bearing type, or attributes to reproduce; public
  visibility and the unmangled FFI item name preserve the header's externally
  callable `ksys_ioperm` contract. The frozen ABI inventory independently
  records the same external C-source declaration for its implementation
  (`rewrite/ABI.tsv`, S000919 `ksys_ioperm` row, source line 71).

No source changes requested.
