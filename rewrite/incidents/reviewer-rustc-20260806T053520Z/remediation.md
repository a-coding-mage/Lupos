# Remediation

1. `B000009` was opened as a Phase 1 blocker without changing resolved Phase 0
   blockers.
2. The exact command, session hash, relevant task evidence hashes, queue hash,
   event-log hash, and source snapshot hashes were retained in this directory.
   The stopped non-review was superseded at its live task path by the required
   replacement review; its exact prior text is retained here as
   `stopped-rust-review.md`, extracted from the immutable session transcript.
3. Reviewer, applier, coordinator, AGENTS.md, and Phase 1 command guardrails
   now forbid direct or delegated compiler/formatter activity and compiler-backed
   rust-analyzer diagnostics. They require an immediate incident on violation.
4. `S016386` retained its candidate and completed parity report. Its stopped,
   unaccepted Rust review was replaced in a fresh isolated context with no
   compiler output or prior stopped-report content. The source-only applier
   ran after the two valid reports and the task reached `DONE`.
5. `S013591` was only defensively paused. It resumed its existing REVIEWING
   stage without evidence invalidation, completed two independent source-only
   reviews and application, and reached `DONE`.
6. Queue state is resumed only through `tools/rewrite_queue.py`, preserving
   attempt history and historical events. Incident lifecycle events are appended
   separately under the queue lock by the incident tool.
7. The live queue, live event log, and live task sources are mutable workflow
   artifacts. The archive now retains the original stopped report, exact two
   candidate files, exact affected PAUSED queue rows, and a byte-verifiable
   append-only event-log prefix so no historical hash is represented as a hash
   of a later mutable file.
8. Independent deep-adjudicator validation passed. `INCIDENT_RESOLVED` was
   appended while holding the queue lock; Phase 0 and the frozen queue remain
   valid, and no active lease remains.
