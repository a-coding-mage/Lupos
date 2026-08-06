# Applier resolution — S000472

Task/lease rechecked: `S000472` is the x86_64-only mapping from
`arch/x86/include/asm/audit.h` to `src/arch/x86/include/asm/audit.rs` on
`feat/bun-like-rewrite-test`.  The pinned revision remains
`425f94c2954b1fe80ebdbf9b29854e89750355df`; the frozen configuration enables
`CONFIG_X86_64`, `CONFIG_AUDIT`, `CONFIG_AUDITSYSCALL`, and
`CONFIG_IA32_EMULATION`.

## Dispositions

1. **Parity P1 / Rust R1 — accepted; task blocked.**  The source declarations
   at `arch/x86/include/asm/audit.h:7-11` are five distinct `extern unsigned
   name[]` incomplete-array declarations.  Their definitions are the mutable
   arrays at `arch/x86/ia32/audit.c:6-29`; the selected x86 consumer passes
   each through C array-to-pointer decay to `audit_register_class` at
   `arch/x86/kernel/audit_64.c:66-70`.  The frozen ABI records for this task
   preserve exactly those incomplete declarations and leave layout/alignment
   `PENDING_REVIEW`; they provide no array bounds.

   A Rust foreign `static mut name: u32` is a scalar object and cannot express
   the C incomplete-array contract, pointer decay, or array-object provenance.
   A fixed `[u32; N]` would invent a public bound that the header intentionally
   does not declare.  `[u32; 0]` would instead declare a complete zero-length
   array, with zero size and no source-backed element extent; it likewise is
   not this ABI.  Rust has no foreign incomplete-array type from which a raw
   element pointer can be derived while retaining this declaration's object
   contract.  No exact representation is available from the pinned source and
   frozen ABI evidence, so the task must be `BLOCKED`, not completed with a
   scalar or fabricated array binding.

2. **Parity P2 — fixed.**  The candidate provenance SPDX identifier now exactly
   preserves the upstream `GPL-2.0` identifier from `audit.h:1`.  This does not
   resolve the array ABI blocker.

3. **Function declaration — verified.**  `ia32_classify_syscall(unsigned int)
   -> int` maps to the frozen x86_64 C ABI widths `u32 -> i32`; its symbol
   spelling and C calling convention remain correct.  No caller, ownership,
   locking, RCU, refcount, or cleanup behavior is present in this declaration
   file beyond the static-lifetime external table objects described above.

## Pending-record closure

For all five table records (`ia32_dir_class`, `ia32_write_class`,
`ia32_read_class`, `ia32_chattr_class`, and `ia32_signal_class`), the final
semantic conclusion is: mutable externally defined `unsigned` array of static
storage duration; bounds intentionally incomplete in this header; ownership is
the defining `arch/x86/ia32/audit.c` object; selected consumers receive only a
decayed pointer; no locking/RCU/refcount contract is introduced here.  The
unrepresentable incomplete-array foreign ABI is the sole blocker.  Include
guards and the `_ASM_X86_AUDIT_H` macro have no Rust runtime or ABI analogue.

No compiler, formatter, analyzer, linker, test, debugger, or runtime command
was run.
