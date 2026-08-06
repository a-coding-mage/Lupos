# S000805 implementation

Source: `vendor/linux/arch/x86/include/uapi/asm/vmx.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The leased queue row is x86_64-only, has no task dependencies, and was verified
as `IN_PROGRESS` for P01 / `p01-terra-fallback`.  The source has no conditional
preprocessor branches, types, functions, storage, or layout-bearing ABI.

Translated all 72 direct value macros with their C integer categories preserved:
the two hexadecimal exit-reason flags are `u32`; the decimal exit-reason and
abort-code macros are `i32`.  The two C initializer-list macros become immutable
typed Rust arrays: `VMX_EXIT_REASONS` contains the exact 65 source pairs and
`VMX_EXIT_REASON_FLAGS` contains the one source pair.  `OTHER_SMI` and
`SEAMCALL` intentionally remain direct constants but are absent from the source
mapping list, matching the original macro definitions.

Read-only inventory checks found 72 direct source macros and 72 corresponding
candidate constants, plus 65 source mapping pairs and 65 candidate pairs.
No compiler, formatter, build, test, linker, or runtime command was run.
