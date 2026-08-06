# S016214 implementation

Source: `vendor/linux/include/uapi/linux/kdev_t.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete selected UAPI header is conditional on `!__KERNEL__` and defines
only `MAJOR(dev)`, `MINOR(dev)`, and `MKDEV(ma, mi)`.  The Rust source maps each
to a scoped expression macro with the pinned shifts, mask, parentheses, and
operator ordering: `dev >> 8`, `dev & 0xff`, and `(ma << 8) | mi`.

Expression macros retain caller-selected operand width and signedness and
evaluate each macro argument once.  The source defines no types, layouts,
objects, or callable ABI; no task-specific row exists in `ABI.tsv`,
`LIFETIMES.tsv`, or `DRIVER_ABI.tsv`.
