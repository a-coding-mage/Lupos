# Reviewer rustc policy incident — B000009

- Opened: `2026-08-06T05:35:20.000Z`
- Incident ID: `B000009`
- Task active at invocation: `S016386` (`src/include/uapi/linux/socket.rs`)
- Reviewer role: Rust reviewer, `gpt-5.6-terra`, high reasoning effort
- Command fragment: `rustc --print sysroot`
- Session record: `/home/fenhir/.codex/sessions/2026/08/06/rollout-2026-08-06T12-16-19-019fd512-6057-70d0-bd49-9a55991751fa.jsonl`

This directory preserves the retained-evidence audit. It does not replace
historical task evidence or events. The source of truth for the incident
classification is `classification.md`; hashes and retention locations are in
`evidence-index.tsv`. `stopped-rust-review.md` is the exact stopped report
extracted from the immutable session transcript after the live task report was
superseded by the required fresh review; it hash-matches the pre-remediation
record listed in the index.

The evidence index intentionally distinguishes immutable retained snapshots
from live mutable workflow files.  The pre-remediation event log is retained as
the first 6,336 append-only records, and the two relevant pre-remediation queue
rows and candidates are immutable files in this directory.
