# S014142 implementation

Oracle: `vendor/linux/include/linux/irqdomain_defs.h` at frozen revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected common header is unconditional for both frozen configurations.
It contains one named C enum tag and sixteen unscoped enumerators, starting at
zero and increasing by one through `DOMAIN_BUS_WIRED_TO_MSI`.

`irq_domain_bus_token` is a transparent `c_int` wrapper: both frozen Kbuild
commands use the ordinary C enum representation (no short-enum option), and
the wrapper preserves the signed `int` ABI and all C object bit patterns.
The enumerators remain unscoped `c_int` constants, matching their C constant
expression scope and the uses in IRQ-domain fields, comparisons, switches, and
bit positions. This header contains no configuration branches, callable code,
ownership, allocation, locking, or cleanup behavior.
