# S000191 applier resolution

## Source basis

- Oracle: `vendor/linux/arch/arm64/include/asm/vdso.h:5-24` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen target/configuration: `aarch64`, with `CONFIG_COMPAT=y`,
  `CONFIG_COMPAT_VDSO=y`, and `CONFIG_THUMB2_COMPAT_VDSO=y`
  (`rewrite/configs/aarch64/frozen.config:498-501`).
- Frozen generated header:
  `rewrite/metadata/aarch64/generated-headers-include-generated.tar`, member
  `include/generated/vdso-offsets.h`, contains exactly
  `#define vdso_offset_sigtramp 0x08d0`.

## Review dispositions

### P1 / R3 — generated offset and token-pasted macro: resolved

The header now materializes the sole frozen generated offset as
`vdso_offset_sigtramp: u64 = 0x08d0`. `VDSO_SYMBOL!` accepts only the selected
identifier `sigtramp`; that identifier has no runtime evaluation. Its base
expression is bound once, then converted through the frozen AArch64 64-bit
`unsigned long` representation and added with `wrapping_add`, returning the C
macro's mutable `void *` equivalent, `*mut core::ffi::c_void`. The result is
therefore not an arbitrary evaluated second operand.

### P1 / R1 / R2 / R4 — linker address objects, provenance, mutability, and FFI safety: resolved

The private zero-size anchors and same-named helper functions were removed.
The header now imports four non-ZST, mutable `u8` address anchors under their
exact linker/source names: `vdso_start`, `vdso_end`, `vdso32_start`, and
`vdso32_end`. This maps the frozen `extern char []` declarations under the
original command family's `-funsigned-char` setting without making a Rust
reference, asserting an image extent, or assigning Rust ownership. Consumers
take raw addresses directly with `core::ptr::addr_of_mut!`; read-only users
perform their own raw const conversion. The local `SAFETY` comment records
that the preserved assembly tasks S000292 (`vdso-wrap.S`) and S000294
(`vdso32-wrap.S`) own and define the labels for the whole kernel-image
lifetime.

### P1 — assembler/preprocessor split: resolved

`__VDSO_PAGES` is retained as the Rust non-assembly constant. The source
header's separate `__ASSEMBLER__` view is still consumed by the original
LINUX_ARCH_ASM linker-script path: S000292 and S000294 are explicitly
classified `LINUX_ARCH_ASM`, and their frozen commands preprocess the pinned
assembly/source headers with `-D__ASSEMBLY__`. The native and compat linker
scripts therefore retain the original macro-only C-preprocessor branch;
generated offsets, C expression macro, and C extern declarations are not
introduced into that assembly path. This Rust file represents only the
non-assembler header interface selected for Rust consumers.

## Pending-record closure for S000191

- `__VDSO_PAGES`: selected fixed integer constant `4`.
- `VDSO_SYMBOL`: selected generated-offset expression; only `sigtramp` exists
  in the frozen generated header; one runtime base evaluation; AArch64
  unsigned-64 wrapping address arithmetic; mutable raw `void *` result.
- Header guard and `__ASSEMBLER__` records: source inclusion mechanics; the
  Rust module is the non-assembler mapping and preserved assembly retains the
  original preprocessor branch.
- `vdso_start`, `vdso_end`, `vdso32_start`, `vdso32_end`: exact external
  C/assembly linker labels with mutable unsigned-byte address semantics;
  static kernel-image lifetime owned by S000292/S000294; no Rust owner,
  reference, drop, lock, RCU, or refcount protocol.

All parity- and Rust-review findings are resolved from frozen source and
metadata. No compiler, formatter, linker, test, emulator, debugger,
benchmark, or rust-analyzer diagnostic was run or used.
