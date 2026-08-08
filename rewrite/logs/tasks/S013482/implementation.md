# S013482 implementation

- Task: `S013482`
- Pipeline/attempt: `P02` / `1`
- Linux source: `vendor/linux/include/linux/audit_arch.h`
- Destination: `src/include/linux/audit_arch.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by both frozen x86_64 and AArch64 configurations)

## Source evidence

Read the complete pinned header and its direct audit contexts:

- `vendor/linux/include/linux/audit_arch.h`
- `vendor/linux/include/linux/audit.h`
- `vendor/linux/lib/compat_audit.c`
- `vendor/linux/lib/audit.c`
- `vendor/linux/kernel/auditsc.c`
- `vendor/linux/arch/x86/ia32/audit.c`
- `vendor/linux/arch/arm64/kernel/ptrace.c`

The header has no configuration branches beyond its include guard. It defines
the seven sequential audit classification values and the count sentinel,
declares the C classifier with `int`/`unsigned int` arguments, and declares
five externally defined `unsigned int` compatibility-class arrays. The arrays
are defined in `vendor/linux/lib/compat_audit.c`; the classifier is defined in
the same file. The definitions are consumed by `vendor/linux/lib/audit.c` and
the audit syscall paths.

## Translation decisions

- `auditsc_class_t` is an explicit `#[repr(C)]` enum with the source order and
  values preserved; the enumerators are re-exported at module scope as in the
  Linux header's shared identifiers.
- The C classifier is declared as an `extern "C"` function with `c_int` and
  `u32` parameters/return, preserving the C ABI and unsigned-int width.
- The five incomplete C array declarations are represented as external
  zero-length `[u32; 0]` declarations. This preserves the symbol address and
  element type while retaining the incomplete-array declaration boundary; the
  storage and length remain owned by the defining Linux object.
- No architecture-specific `cfg` was introduced: the frozen scope marks this
  header `common`, and both architecture consumers use the same declaration.
- No implementation, test, stub, or generated module index was added outside
  the leased destination.

## Safety

The FFI declarations are unsafe to call or access and retain the Linux
external-object lifetime and synchronization contract. No Rust references or
ownership claims are introduced by this header translation.
