# Rust review — S000472 (slot 2)

Reviewed only the pinned source and Rust candidate; no compiler, formatter,
rust-analyzer, linker, test, debugger, or runtime tool was invoked.

## Scope verified

- Branch: `feat/bun-like-rewrite-test`.
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Queue row: `S000472` is `REVIEWING`, maps
  `arch/x86/include/asm/audit.h` to
  `src/arch/x86/include/asm/audit.rs`, and is x86_64-only.
- Frozen x86_64 configuration enables `CONFIG_X86_64`, `CONFIG_AUDIT`,
  `CONFIG_AUDITSYSCALL`, and `CONFIG_IA32_EMULATION`
  (`rewrite/configs/x86_64/frozen.config:62,64,314,690`).

## Finding R1 — high: foreign class-table declarations change array objects into scalar objects

The five C declarations are incomplete arrays of `unsigned`, not scalar
objects: `vendor/linux/arch/x86/include/asm/audit.h:7-11`. Their definitions
are arrays in `vendor/linux/arch/x86/ia32/audit.c:6-29`, and the selected x86
consumer relies on C array-to-pointer decay when it passes each table to
`audit_register_class` (`vendor/linux/arch/x86/kernel/audit_64.c:66-70`).

The candidate instead declares each symbol as `pub static mut ...: u32`
(`src/arch/x86/include/asm/audit.rs:19-23`) and documents that it represents
only the first element (`:9-14`). `u32` has the right element width for the
frozen x86_64 `unsigned`, and the unmangled `extern "C"` import will name the
same linker symbol, but a Rust scalar foreign static is not the C array object
declared by this header. It creates a public unsafe API that permits a direct
scalar read/write and a `*mut u32` provenance rooted in a single scalar rather
than in the complete foreign array. The documentation cannot restore the lost
array type or prevent those operations.

Represent each declaration as an incomplete/zero-length foreign array binding
(the Rust representation used for a C incomplete array), and require raw
address conversion to the element pointer at the call boundary. That preserves
the array-object declaration and makes accidental indexing or scalar access
unavailable. The applier must re-review all five bindings and any future Rust
caller of the tables.

## Checked without finding

- `ia32_classify_syscall`: C `unsigned int` to `u32` and C `int` to `i32` are
  width- and signedness-correct for the frozen x86_64 ABI
  (`audit.h:5`; `audit.rs:17`).
- The candidate's `unsafe extern "C"` import block correctly keeps calls and
  access to mutable foreign statics unsafe, and no `unsafe` operation is
  performed within this declaration-only file (`audit.rs:16-23`).
- No Rust consumer of these six declarations currently exists outside this
  candidate, so R1 is localized but must be resolved before adding one.

## Verdict

Reject pending resolution of R1. No source files or queue state were edited.
