# S014142 application resolution

## Decision: BLOCKED

The two approved frozen configurations select the same unconditional header and
IRQ-domain/MSI consumers.  `include/linux/irqdomain_defs.h:12-29` establishes
the sixteen ordinal values, but does not establish the storage size, alignment,
or signedness of `enum irq_domain_bus_token`.

The identity-bound ABI rows for this exact type remain `PENDING_REVIEW` for
both `aarch64` and `x86_64` (`rewrite/ABI.tsv`, rows S014142), and the paired
ownership/lifetime rows also remain pending.  The Phase 0 header-closure
metadata proves selection on both targets, but its Kbuild command inventories
contain no target type-layout or enum-signedness result.  The absence of
`-fshort-enums` in those commands is not an identity-bound proof of the enum
object ABI.  Therefore the candidate's `c_int` carrier and its claim of a
frozen signed-`int` ABI cannot be accepted.

The exact unknown representation matters: this tagged enum occurs in the
`irq_domain` and `irq_domain_info` fields and in callback/function parameters
in `include/linux/irqdomain.h`, and in `msi_domain_info` in
`include/linux/msi.h`.  An incorrect carrier changes those layouts and the
interpretation of values received through those interfaces.

## Review dispositions

### P1 — confirmed, not resolved by this candidate

The transparent newtype is distinct from `c_int`, while all sixteen exported
`DOMAIN_BUS_*` names are `c_int`.  Pinned C consumers pass these enumerators
directly to tagged-enum parameters (`include/linux/irqdomain.h:406-408`,
`kernel/irq/ipi-mux.c:186`, and `arch/x86/kernel/hpet.c:565`) and use them as
integer operands (`include/linux/irqchip/irq-msi-lib.h:13,18`).  Rust has no
implicit conversion from the constants to the newtype.  No frozen, complete
inter-file conversion rule establishes where equivalent conversions would be
made, so the distinct-wrapper API cannot be accepted as source-parity.

### R1 — confirmed and blocking

No identity-bound, target-specific ABI evidence proves the C enum's size,
alignment, or signedness for either frozen target.  The task remains blocked
until Phase 0 provides that evidence and the ABI rows can be resolved; Phase 1
does not run a compiler or create new ABI probes.

### R2 — resolved

The candidate and its snapshot now retain the upstream SPDX identifier exactly
as `GPL-2.0`.  `rewrite/BRANDING_ALLOWLIST.tsv` contains no license exception.

No compiler, formatter, linker, test, runtime command, or diagnostic was run.
