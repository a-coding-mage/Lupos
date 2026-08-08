# S013482 application resolution — BLOCKED

Task: `S013482` (`include/linux/audit_arch.h` ->
`src/include/linux/audit_arch.rs`), attempt 1, pipeline `P02`.

The current candidate and sealed semantic-closure proposal are not accepted.
No source was changed during application.  The task is blocked because the
candidate changes the pinned header's interface and its frozen selected-symbol
closure omits a declared symbol.  The Phase 1 queue and manifests are frozen,
so application cannot add the missing closure record or introduce an
unreviewed replacement representation.

## Evidence reopened independently

- Pinned Linux `425f94c2954b1fe80ebdbf9b29854e89750355df`:
  `include/linux/audit_arch.h:12-31` in full; direct definitions and callers in
  `lib/compat_audit.c:7-56`, `lib/audit.c:40-88`,
  `arch/x86/ia32/audit.c:31-48`, and `kernel/auditsc.c:152-193`.
- Frozen S013482 records in `SYMBOLS.tsv` (34 rows), `ABI.tsv` (12 rows), and
  `LIFETIMES.tsv` (12 rows), plus the sealed S013482 proposal and both semantic
  review attestations.  None contains a S013482 record for
  `audit_classify_compat_syscall`.
- The distinct definition task S017105 records that function only for AArch64
  (`lib/compat_audit.c:32`), and its frozen queue row depends on S013482.  It
  does not repair the absent common-header declaration record or authorize a
  Phase 1 inventory change.

Slot 1 disclosed cross-task-log exposure after completing its report.  It was
therefore not used as an acceptance basis.  The dispositions below were made
from the pinned sources and frozen records above; the slot-1 findings served
only as leads.  Slot 2 was considered only as corroboration, not as a
substitute for that source review.

## Finding dispositions

### P01 — incomplete external arrays became `[u32; 0]`

**Disposition: sustained; blocking.**

`audit_arch.h:27-31` declares five `extern unsigned int name[]` objects with
no bound.  The candidate instead gives all five a `[u32; 0]` type.  This
injects a zero bound where the source deliberately has none.  The selected
AArch64 definition at `lib/compat_audit.c:7-30` gives each symbol a nonempty,
architecture-generated initializer ending in `~0U`, and `lib/audit.c:74-79`
passes each base address to `audit_register_class`, whose declared parameter
is `unsigned int *` in `include/linux/audit.h:136`.  A zero-length Rust array
cannot represent those elements or that pointer-use contract.

No correction was applied: the candidate and proposal are sealed, and the
frozen ABI/lifetime records still leave the external-array representation and
access contract `PENDING_REVIEW`.  Encoding a scalar/pointer façade or an
invented extent would be a new, unreviewed design rather than a source-proven
one.

### P02 — C enumerator integer/ordinary-namespace contract changed

**Disposition: sustained; blocking.**

The source enum at `audit_arch.h:12-21` supplies the ordinary C enumerators
`AUDITSC_*` with consecutive values 0 through 7.  Direct selected code returns
them through `int`: `lib/compat_audit.c:32-55`, `lib/audit.c:40-70`, and
`arch/x86/ia32/audit.c:31-48`.  `kernel/auditsc.c:159-193` then switches on the
integer classifier result using the same bare values.  The candidate instead
exports Rust enum variants (and re-exports those variants), changing each
name's expression type to `auditsc_class_t` and adding a scoped enum-variant
interface absent from the C header.

No correction was applied.  Replacing the values with `c_int` constants alone
would leave the header's enum-tag representation unresolved; retaining the
Rust enum retains the observed incompatible value domain.  The sealed proposal
cannot establish a faithful combined Rust representation without a reviewed
source-level design, so application must not guess.

### P03 — classifier declaration omitted from S013482 frozen closure

**Disposition: sustained; blocking.**

`audit_arch.h:24` declares `audit_classify_compat_syscall(int, unsigned int)`;
the source definition at `lib/compat_audit.c:32-56` and the call at
`lib/audit.c:40-44` confirm it is operative for the selected AArch64 generic
compat path.  Yet the complete frozen S013482 symbol/ABI/lifetime record sets
contain only the enum and five arrays.  The sealed proposal has a scope-status
entry but no record key for this declared function.  S017105's definition-side
records cannot close this missing header-side, common-architecture closure.

Adding that record would mutate frozen Phase 0 inventory during Phase 1.
Accordingly the declaration's selected ABI/lifetime/linkage record cannot be
closed, and `DONE` is prohibited.

### RUST-ENUM-INT-DOMAIN

**Disposition: sustained; same blocking defect as P02.**

Independent source review above confirms the integer return/switch use and
the candidate's enum-typed replacement.  No separate correction is applied
for the reasons recorded under P02.

### RUST-INCOMPLETE-ARRAY-ABI

**Disposition: sustained; same blocking defect as P01.**

Independent source review above confirms nonempty generated definitions and
pointer registration use; `[u32; 0]` is not an incomplete C array declaration.
No separate correction is applied for the reasons recorded under P01.

## Final application result

S013482 must transition from `APPLYING` to `BLOCKED`.  Required future work is
a controlled scope/closure correction followed by a fresh reviewed candidate;
it is not a Phase 1 source-only patch.  No compiler, formatter, linker, test,
runtime tool, or historical Lupos Rust source was used.
