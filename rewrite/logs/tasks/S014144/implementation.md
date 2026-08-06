# S014144 implementation

Source oracle: `vendor/linux/include/linux/irqflags_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header has one guarded declaration: `struct irqtrace_events`, under
`CONFIG_TRACE_IRQFLAGS`. Neither frozen configuration defines that symbol.
Both only define `CONFIG_TRACE_IRQFLAGS_SUPPORT=y` and
`CONFIG_TRACE_IRQFLAGS_NMI_SUPPORT=y`; these are distinct Kconfig symbols and
do not enable the guarded declaration. The selected x86_64 and AArch64 header
therefore has no effective declarations or ABI objects. The destination keeps
only required immutable provenance and documents this selected conditional
state. No included type or macro is needed by the active branch.

Direct consumers confirm the same condition: `include/linux/sched.h` embeds
`irqtrace_events` only under `CONFIG_TRACE_IRQFLAGS`, and
`include/linux/irqflags.h` includes this header but has no active type use when
the symbol is absent.
