# S000686 Rust review (slot 2)

Reviewed `vendor/linux/arch/x86/include/asm/shared/tdx_errno.h` at pinned
revision `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/arch/x86/include/asm/shared/tdx_errno.rs`.

## Finding R1 — operand-ID constants change the C literal type (major)

The four source macros `TDX_OPERAND_ID_RCX`, `TDX_OPERAND_ID_TDR`,
`TDX_OPERAND_ID_SEPT`, and `TDX_OPERAND_ID_TD_EPOCH` on upstream lines 35–38
are unsuffixed integer literals.  Each value fits in a C `int`, so on the
frozen x86_64 target each macro expands as a signed `int`; it is then subject
to C's usual arithmetic conversions at each use site.  Candidate lines 33–36
instead permanently give the constants type `u32` based only on the comment
that the detail information occupies bits 31:0.

This changes signedness and the default integer-promotion/conversion behavior.
For exact source semantics, represent these four literal macros as `i32`
constants (their source literal type) and require any translated use which
needs a different width or unsigned operation to perform the corresponding
explicit conversion at that use site.  The documented bit position does not
alter the C literal's type.

## Checked without finding a defect

- `TDX_SEAMCALL_STATUS_MASK` and all 19 status-code macros carry `ULL` in the
  pinned header; all corresponding candidate constants are `u64` with the
  exact same 64-bit bit patterns.
- The source has no configuration-controlled status-code branch: only a C
  include guard.  The Rust module has no inappropriate `cfg` gating.
- The source has no storage, FFI layout, ownership, aliasing, unsafe,
  allocation, panic, or cleanup behavior.

No compiler, formatter, build, test, or runtime command was run.
