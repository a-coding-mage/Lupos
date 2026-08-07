# Parity review — S014598 / attempt 1 / slot 1

Result: **FINDINGS**.  This is a source-only review; no compiler, formatter,
linker, test, rust-analyzer diagnostic, or Git command was used.

Reviewed inputs were the current candidate `src/include/linux/pci_ids.rs`, the
pinned `vendor/linux/include/linux/pci_ids.h`, both frozen configurations, the
current Phase-0 identity/queue fingerprint, and the sealed
`semantic-closure-proposal.tsv`.

## Finding F001 — sealed semantic-closure evidence does not bind the current candidate

- Linux symbol/interface and local source evidence: `_LINUX_PCI_IDS_H` at
  `vendor/linux/include/linux/pci_ids.h:10-11` encloses the complete, active
  PCI-ID macro interface, from `PCI_CLASS_NOT_DEFINED` at line 15 through
  `PCI_VENDOR_ID_NCUBE` at line 3268, terminated at line 3270.  The header has
  no configuration branches: its only non-`#define` directives are that guard
  at lines 10 and 3270.  The complete active interface is therefore the 2,902
  unconditional object-like macros under this guard.
- Candidate/source result: the current candidate has exactly 2,902 `pub const
  NAME: i32 = VALUE;` declarations.  Name/value comparison against all 2,902
  active Linux `#define`s is exact; there are no missing, extra, or
  differently-valued declarations.  All Linux RHSs are unsuffixed numeric
  literals, all are within signed 32-bit range (maximum `0x0d1010`,
  `PCI_CLASS_WIRELESS_WHCI`, source line 135), so the explicit `i32` mapping
  preserves their C `int` value and promotions in the selected configurations.
  The source macros have no linkage or object layout; the candidate likewise
  introduces no exported ABI symbol or layout.  Provenance names the pinned
  source, SHA `425f94c2954b1fe80ebdbf9b29854e89750355df`, `common`, and task
  `S014598`.  The guard's language-specific non-emission is recorded as
  `NOT_APPLICABLE` by the two frozen selection records cited below, rather than
  an unreviewed feature/config omission.
- Evidence failure: every one of the proposal's 11,617 records, including the
  task scope record, binds candidate SHA-256
  `828fb8678dd9f116da63365df7ee1c814ac09e502c64f7c50f6fba9f9fe59e9c`.
  The current reviewed candidate hashes to
  `9e2d27850150685d36b9f50f232ae4594903467f06641a3f337acb532085ac51`.
  Thus the sealed closure is not evidence for the candidate actually reviewed,
  even though its present macro mapping is source-equivalent.  The closure
  completion cannot be attested or used to complete slot 1 until it is
  regenerated/resealed against this exact candidate (or the candidate is
  restored by an authorized stage).
- Closure finding-key binding: `SC1-63e16b9d32b57fa9035a58a16758551c034e2f995590e3b8f84fef0fbfccd4f9`
  (the sole task scope record).  Related guard records:
  `SC1-8fe5caf89d71ced9219128329ea229853bc263ddb2d5a0d5e425667b5c5474c0`
  (aarch64 selection) and
  `SC1-c9bbd44067140d83674a52fea7b537e89d9d4adb3ed615989b7cb531ba83ec67`
  (x86_64 selection).

## Exhaustive comparison notes

- The proposal has 11,617 records: 11,612 operative-macro records, four guard
  conditional records, and one scope record.  Its cited Linux SHA, Phase-0
  identity SHA (`0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`),
  queue fingerprint (`cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`),
  and implementation SHA are internally uniform; the candidate SHA above is
  the sole binding mismatch.
- Each operative-macro proposal record's symbol and cited source line matches
  the matching Linux `#define`; all records carry a source citation.  The
  candidate value/name mapping is exact across the full source set.
- No locks, allocation, ordering, error path, refcount, RCU, linkage, layout,
  callable symbol, configuration-controlled data branch, or branding delta is
  present in this definitions-only header beyond the guard handled above.
