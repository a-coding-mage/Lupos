# S000767 implementation

Source oracle: `vendor/linux/arch/x86/include/asm/xen/trace_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The x86_64 frozen configuration has `CONFIG_XEN` disabled, but the complete
Phase 0 inventory selects this unconditional header through the Xen trace-event
header closure for `arch/x86/kernel/callthunks.o`.  There are no source-level
configuration branches in the oracle header.

Mapped all three selected declarations:

- `enum xen_mc_flush_reason` is a C-representation Rust enum with the four
  explicitly numbered C enumerators in declaration order.
- `enum xen_mc_extend_args` is a C-representation Rust enum with the three
  explicitly numbered C enumerators in declaration order.
- `xen_mc_callback_fn_t` is a nullable C-ABI callback function pointer taking
  the C `void *` equivalent, preserving C null function-pointer representation.

The direct trace-event consumer stores both enums as trace fields and stores
the callback pointer as a function-pointer field; `arch/x86/xen/multicalls.c`
emits the defined enum values.  This task contains no functions, state,
allocation, locking, or cleanup paths.
