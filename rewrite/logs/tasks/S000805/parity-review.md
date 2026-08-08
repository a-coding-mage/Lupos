# S000805 parity review — slot 1

Review result: **FINDINGS**.

Reviewed candidate binding:

- task/attempt/pipeline: `S000805` / `1` / `P02`
- candidate-diff SHA-256: `ecce5e2cf9ed4fd4b561a9cf6dca8db733f61f1b33d1c611394f8a447b0cd6a0`
- implementation SHA-256: `a2fd9d3b9cd2636fb5536fc272c657191cee3bd16077411fbb79fb6e4fecb655`
- sealed proposal SHA-256: `5ad8ab2552865ec6902bab18697a7fd96812bf7d66cea91899638722562ebd97`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Phase-0 identity SHA-256: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`

## P1 — initializer-fragment macros became whole array expressions and lose defining-module bindings

Affected proposal records:

- `SC1-b477b1b4eff2bf6beb9760d8a18c595e5c2374e61cbb584c5ef60c07af731757`
  (`VMX_EXIT_REASONS`, `selection_expression`)
- `SC1-2cd13e727c572cb4de23adcd377d70827a7d2974cc6590599d4d0510cc4e4303`
  (`VMX_EXIT_REASONS`, `status`)
- `SC1-677985eba31a18d00d28e63138e00463220b14e78ed809aa1b5205f2e8acb85a`
  (`VMX_EXIT_REASON_FLAGS`, `selection_expression`)
- `SC1-28bbfbf5d026135dce882a9fbbbbb1f6b67ec46ab9cca652144b6e5fe3fe466c`
  (`VMX_EXIT_REASON_FLAGS`, `status`)

Pinned source `vmx.h:101-166` defines `VMX_EXIT_REASONS` as a comma-separated
sequence of brace initializers, with no enclosing array.  `vmx.h:168-169` does
the same for `VMX_EXIT_REASON_FLAGS`.  Those definitions are initializer-list
fragments intended to be inserted into a caller-owned C aggregate initializer.
The candidate instead exports `VMX_EXIT_REASONS!()` as one complete `[ ... ]`
array at `vmx.rs:81-152` and `VMX_EXIT_REASON_FLAGS!()` as one complete array
at `vmx.rs:154-159`.  Therefore the Rust expansions cannot occupy the original
fragment position inside a caller's aggregate initializer; adding the caller's
surrounding brackets would create a nested array instead of the original flat
entries.  This changes the selected macros' expression category and their
call-site ABI/source contract.

Additionally, the two `#[macro_export]` expansions use unqualified constant
names (for example `EXIT_REASON_EXCEPTION_NMI` at `vmx.rs:85` and
`VMX_EXIT_REASONS_FAILED_VMENTRY` at `vmx.rs:157`).  In an exported
`macro_rules!` expansion those paths are resolved at the invocation site, not
as defining-module items.  A consumer that merely invokes the public macro
does not receive the source header's defining-include binding and must instead
have matching local names in scope.  The C macro expansions reference the
header-defined macros after preprocessing and have no analogous caller-scope
precondition.  Preserve the initializer-fragment contract (or record a
verified caller-compatible Rust representation) and bind every referenced
constant to the defining module without changing the expansion category.

No other parity discrepancy was found by manual source comparison: all 65
numeric exit-reason macros, both flag values, all three abort values, and the
source list's intentional exclusions (`EXIT_REASON_OTHER_SMI` and
`EXIT_REASON_SEAMCALL`) are retained with their source order and values.
