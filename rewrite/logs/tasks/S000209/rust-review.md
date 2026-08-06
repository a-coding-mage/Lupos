# Rust review — S000209

## Finding R1 — `u64` changes each UAPI macro's direct C expression type (must fix)

`AT_SYSINFO_EHDR`, `AT_MINSIGSTKSZ`, and `AT_VECTOR_SIZE_ARCH` in
`arch/arm64/include/uapi/asm/auxvec.h` are respectively the unsuffixed decimal
C integer constants `33`, `51`, and `2`.  On the frozen AArch64 target each is
a signed `int` expression, not an `unsigned long`/auxv-word expression.  The
candidate publishes all three as `u64`, changing the public Rust constant API,
the type and overflow/signedness rules of expressions which use the constants,
and when an explicit conversion is required.

The pinned consumers confirm that widening belongs at their use sites: ARM64
`ARCH_DLINFO` passes the keys to `NEW_AUX_ENT`, whose assignments convert them
to the `elf_addr_t` saved-auxv word in `fs/binfmt_elf.c`; `AT_VECTOR_SIZE_ARCH`
participates as an `int` expression in `AT_VECTOR_SIZE`, which then becomes an
array extent for `mm_struct::saved_auxv`.  The constant definitions therefore
must retain their direct C expression type (use `i32` for these values on the
frozen target), with any needed word/`usize` conversion made by the matching
consumer rather than by this UAPI definition.

Evidence: `vendor/linux/arch/arm64/include/uapi/asm/auxvec.h:21-24`,
`vendor/linux/arch/arm64/include/asm/elf.h:167-182`,
`vendor/linux/fs/binfmt_elf.c:233-248`, and
`vendor/linux/include/linux/mm_types.h:29-32,1293`.

## Checks with no additional finding

- The candidate retains the source SPDX identifier exactly, including
  `Linux-syscall-note`, retains the ARM copyright notice, and carries the
  required immutable source/revision/architecture/task provenance.  There is
  no licensing or provenance mismatch.
- This header defines only constants: it has no C object, function, structure,
  union, bitfield, linkage, calling convention, or FFI layout to mirror.
- The frozen AArch64 configuration enables `CONFIG_COMPAT` and
  `CONFIG_COMPAT_VDSO`; its compatibility `ARCH_DLINFO` still consumes the
  `AT_SYSINFO_EHDR` key as the same C `int` macro before the consumer converts
  its value to the auxv-word representation.

## Verdict

Reject pending resolution of R1.  No compiler, formatter, linker, test, or
runtime diagnostic was invoked.
