# Applier resolution — S000758 (P02, attempt 1)

## Evidence reopened

- Pinned source: `vendor/linux/arch/x86/include/asm/vmxfeatures.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, read in full.
- Candidate: `src/arch/x86/include/asm/vmxfeatures.rs` and the task-local
  `candidate.diff`.
- Task evidence: `implementation.md`, `parity-review.md`, and
  `rust-review.md`.
- Frozen task records: `S000758` rows in `rewrite/SCOPE.tsv`,
  `rewrite/SYMBOLS.tsv`, `rewrite/ABI.tsv`, and `rewrite/LIFETIMES.tsv`;
  the queue row remains `REVIEWING`.
- Narrow pinned consumer contexts: `asm/processor.h`, `asm/vmx.h`, and
  `arch/x86/kernel/cpu/feat_ctl.c`.

The pinned header has 65 object-like value definitions (`NVMXINTS` and 64
`VMX_FEATURE_*` definitions), and the candidate has the same 65 definitions.
The frozen symbol inventory also records the include guard and its closing
conditional; the Rust module boundary is the source-supported analogue.  This
does not alter the disposition below.

## Finding dispositions

### RUST-001 — SPDX identifier changed

**Disposition: ACCEPTED — source correction required.**

Pinned upstream line 1 is exactly
`/* SPDX-License-Identifier: GPL-2.0 */`.  Candidate line 1 and the immutable
candidate snapshot instead state `// SPDX-License-Identifier: GPL-2.0-only`.
The frozen source is the controlling implementation oracle, and the rewrite
protocol requires upstream SPDX identifiers to be retained; no branding
allowlist entry authorizes changing this identifier.  The source correction is
therefore to retain `GPL-2.0` verbatim in the Rust provenance line.  No source
edit was made by this applier.

## Review/evidence disposition and recommended queue outcome

This task must not enter `APPLYING` or `DONE` from the current evidence.  The
candidate requiring correction is not the candidate approved by the existing
parity report, and the slot-2 review cannot serve as a reliable acceptance
attestation for a corrected candidate.  A source-proven correction must be
made through a controlled requeue, with a fresh candidate snapshot and fresh
independent parity and Rust reviews of that exact corrected file before a later
applier adjudicates it.

**Recommended queue outcome: controlled requeue to implementation, not
`APPLYING` and not `DONE`.**  The requeue must preserve the present evidence as
the rejected attempt, apply only the required SPDX correction to the leased
destination, regenerate `implementation.md` and `candidate.diff`, and obtain
two reliable independent review attestations before any future apply stage.

No compiler, formatter, linker, test, runtime tool, or historical Rust source
was used for this resolution.
