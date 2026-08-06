# S014145 implementation

Source: `vendor/linux/include/linux/irqhandler.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header contains one selected ABI item: C typedef
`void (*irq_flow_handler_t)(struct irq_desc *desc)`.  The Rust alias retains a
nullable C-ABI function pointer (`Option<unsafe extern "C" fn>`) and a mutable
raw pointer to the descriptor.  The descriptor identity is imported from the
separately inventoried `include/linux/irqdesc.h` mapping rather than represented
by a distinct opaque type, because the C forward declaration and that later
definition name the same tagged struct.

The C include guard has no runtime Rust item.  There are no configuration
conditionals, storage objects, functions, locking, ownership transfer, or
cleanup paths in this header.  No compiler, formatter, test, or runtime command
was invoked.
