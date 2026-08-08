# S016888 applier resolution — attempt 1

Pinned source: `vendor/linux/kernel/locking/lock_events_list.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## P1-LOCK_EVENT-MACRO-CONTRACT — ACCEPTED; unresolved / BLOCKED recommended

The finding is correct.  The upstream header makes the `LOCK_EVENT` contract
operative, rather than using it as a convenience for storing event names:

- Lines 16-18 install the default, one-argument expansion
  `LOCKEVENT_ ## name,` only when the includer has not supplied `LOCK_EVENT`.
- Each selected item at lines 44-102 is a separate `LOCK_EVENT(name)`
  expansion in source order.
- `kernel/locking/lock_events.h:16-25` includes this header under the default
  expansion to create the `enum lock_events` members.
- `kernel/locking/lock_events.c:22-28,46-50` undefines and redefines that
  same macro as `[LOCKEVENT_ ## name] = #name,`, then includes the list again
  to create one designated initializer per event.

The candidate's `lock_events_list!` makes one call with a comma-separated
identifier aggregate.  It therefore neither performs one callback expansion
per event nor implements the upstream default/override inclusion contract.
Correct event ordering does not repair that semantic difference.

I did not change the candidate.  Replacing its aggregate callback with a
per-item callback would still leave the required default `LOCKEVENT_ ## name`
identifier construction and includer-controlled redefinition unresolved in
this task's sole, path-preserving Rust file.  A speculative replacement API
would be a new unreviewed design, not a source-proven faithful mapping.

## RUST-1 — ACCEPTED; unresolved / BLOCKED recommended

The Rust review identifies the same source-level contract loss and is also
correct.  The frozen one-file mapping supplies no source-proven Rust
representation that both preserves the header's conditional default and lets
each local consumer replace the same one-identifier transformation before a
separate expansion, including the token-pasted enum identifier and the
stringified designated-initializer form required by the two pinned consumers.
The current macro instead requires a distinct list-accepting variadic
callback, which is incompatible with both of those upstream consumers.

The current semantic-closure proposal is sealed to candidate SHA
`706ee6e55b4da4f99942fa0196fb825889413bf3c6780f2a67d357d8a14a70dd` and
proposes `COMPLETE` for the `LOCK_EVENT` records despite this unresolved
contract.  No final semantic closure may be committed from that proposal.

## Disposition and handoff

Do not mark this attempt `DONE`.  Do not apply the existing semantic-closure
proposal or review seals as a closure of the `LOCK_EVENT` records.  Source-only
evidence establishes the defect but does not establish a faithful correction
within the frozen one-file mapping.  The coordinator should transition
S016888 to `BLOCKED` with this macro-contract reason; no candidate or queue
state was changed by this applier action.

No compiler, formatter, linker, test, runtime tool, or historical Lupos source
was used.
