# S014142 implementation

BLOCKED before source creation. The pinned `include/linux/irqdomain_defs.h`
declares `enum irq_domain_bus_token` with sixteen ordinal enumerators, but the
identity-bound `rewrite/ABI.tsv` rows for both frozen x86_64 and AArch64 remain
`PENDING_REVIEW` for its size, alignment, and signedness. The frozen header and
Kbuild command evidence do not establish that object ABI.

This is ABI-critical: the enum is embedded in IRQ-domain/MSI structures and is
used in callback and function parameter contracts by the pinned
`include/linux/irqdomain.h` and IRQ-domain consumers. A Rust representation
would therefore guess at layout and calling behavior. No destination source
was written.

No compiler, formatter, linker, test, runtime, benchmark, or historical Rust
source was used.
