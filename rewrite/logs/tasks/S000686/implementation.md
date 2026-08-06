# S000686 implementation

- Source: `arch/x86/include/asm/shared/tdx_errno.h` at pinned revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/arch/x86/include/asm/shared/tdx_errno.rs`.
- The source is an unconditional x86_64 header of 24 integer macros: one RAX status mask, 19 SEAMCALL status codes, and four error-detail operand IDs.
- Each status-code macro is mapped to a public `u64` constant, matching the source's `ULL` width and its use as an architectural RAX value. The unsuffixed operand-ID literals are public `i32` constants, preserving their frozen C `int` category; their documented bits 31:0 placement does not change their literal type.
- Numeric values, names, and source comments describing RAX and bits 31:0 are retained. No configuration condition, storage, control flow, unsafe operation, allocation, test, or fallback behavior exists in the source.
- No ABI, lifetime, locking, driver, or unresolved semantic record applies to this constants-only header.
