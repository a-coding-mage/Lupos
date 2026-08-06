# S013505 implementation record

Source: `vendor/linux/include/linux/bcma/bcma_regs.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete 104-line selected common header contains 75 object-like `BCMA_`
macros and no functions, types, storage, layout declarations, linkage, locking,
ownership, lifetime, or conditional configuration behavior.  Its include guard
is represented by Rust module inclusion rather than an exported item.

`src/include/linux/bcma/bcma_regs.rs` preserves every macro name and numeric
value as a public `u32` constant.  These values are BCMA register offsets,
register masks, shifts, PCI configuration offsets, and 32-bit backplane
addresses; their in-scope consumers use them with 32-bit register and address
operations.  `BCMA_SOC_PCI_MEM_SZ` retains its source expression
`64 * 1024 * 1024`.

No source outside the pinned Linux tree was consulted.  No compiler, formatter,
test, build, or runtime command was run.
