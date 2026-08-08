# S000805 Rust review — slot 2

Task: `arch/x86/include/uapi/asm/vmx.h` → `src/arch/x86/include/uapi/asm/vmx.rs`  
Attempt/pipeline: `1` / `P02`  
Role/model/effort: Rust reviewer slot 2 / `gpt-5.6-terra` / `high`  
Candidate diff SHA-256: `ecce5e2cf9ed4fd4b561a9cf6dca8db733f61f1b33d1c611394f8a447b0cd6a0`  
Semantic-proposal SHA-256: `5ad8ab2552865ec6902bab18697a7fd96812bf7d66cea91899638722562ebd97`

## Result: reject — source correction required

### RUST-S2-001 — list-fragment macros have been replaced by non-equivalent fixed-array expressions

`VMX_EXIT_REASONS` is a C preprocessor replacement list containing 65 comma-
terminated brace initializers, not an array expression.  `VMX_EXIT_REASON_FLAGS`
is likewise a brace initializer fragment.  This lets a C caller splice either
macro into the initializer and select the destination element type in its
surrounding declaration.

The candidate changes those replacement categories to `#[macro_export]`
`macro_rules!` macros that expand to a standalone fixed Rust array
([`vmx.rs:81-159`](../../../../src/arch/x86/include/uapi/asm/vmx.rs)).  That
forces an inferred `[(i32, &str); 65]` / `[(u32, &str); 1]` shape, prevents
splicing the entries into a caller-owned aggregate, and exports the macros at
crate root although their unqualified constant names remain defined in this
module.  A use outside this module therefore cannot rely on the C header's
macro/identifier availability.  These are observable macro-expansion and type
context differences; the source has no struct, layout, or FFI declaration that
would justify the substituted tuple arrays.

Upstream evidence: `vendor/linux/arch/x86/include/uapi/asm/vmx.h:101-169`.
Candidate evidence: `src/arch/x86/include/uapi/asm/vmx.rs:81-159`.
Proposal records: `SC1-b477b1b4eff2bf6beb9760d8a18c595e5c2374e61cbb584c5ef60c07af731757`,
`SC1-2cd13e727c572cb4de23adcd377d70827a7d2974cc6590599d4d0510cc4e4303`,
`SC1-677985eba31a18d00d28e63138e00463220b14e78ed809aa1b5205f2e8acb85a`, and
`SC1-28bbfbf5d026135dce882a9fbbbbb1f6b67ec46ab9cca652144b6e5fe3fe466c`.

Required resolution: replace the fixed-array/export design with an explicitly
documented, caller-context-preserving Rust representation.  Its paths must
resolve the numeric constants at the definition site (or carry them as explicit
arguments) and it must not impose a tuple-array element type where the C macro
does not.  If Rust cannot express the required token-fragment category without
changing selected caller semantics, record that exact source-level limitation
and block rather than retain this convenient substitution.

## Checks without findings

- `VMX_EXIT_REASONS_FAILED_VMENTRY` is correctly `u32` for the x86_64 C
  unsuffixed hexadecimal literal `0x80000000`; the SGX flag and every remaining
  numeric literal fit C `int` and are represented as `i32`.  Relevant proposal
  records: `SC1-06034794370c67c8d6e627723f1c32a746006087a674f26c042eeef6c49add76`,
  `SC1-b72c97e609abf3808c229100db84322a1fdbb93c0df63a9b009dda971f47fb49`,
  `SC1-c9b0517ecc6172ed74014e7d061273155b17adb239c171713afda22a751cf32f`, and
  `SC1-2aa6fb443a750035573a4bb30736adebdacfaa212aaa5e92b16958db16e07564`.
- All 70 source numeric macros are present as 70 typed Rust constants, with the
  original gaps and values retained (`vmx.h:29-99,171-173`; `vmx.rs:10-79,161-163`).
- The header defines no C object layouts, bit-fields, unions, pointers, FFI
  functions, or unsafe lifetime contract.  The candidate introduces none, so
  no `repr(C)`, packing, alignment, pointer, cast, or panic/overflow finding is
  applicable beyond the macro-expression issue above.

No compiler, formatter, linker, test, debugger, emulator, rust-analyzer
diagnostic, or runtime command was used.
