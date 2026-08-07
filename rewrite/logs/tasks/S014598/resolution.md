# Applier resolution — S014598 / attempt 1 / P01

This resolution is source-only. No compiler, formatter, linker, test,
runtime command, rust-analyzer diagnostic, or Git command was used directly.

## F001 — DISPROVED; no source change

The finding assumes that the closure proposal's `candidate_sha256` must equal
the digest of `src/include/linux/pci_ids.rs`. That binding is not defined by
the current frozen closure tool. `fixed_task_paths` identifies `candidate` as
`rewrite/logs/tasks/S014598/candidate.diff` (`tools/semantic_closure.py:524-538`),
and `proposal_metadata` assigns `candidate_sha256` as
`sha256_file(paths["candidate"])` (`tools/semantic_closure.py:548-565`).
`validate_sealed_proposal` checks the same candidate evidence path
(`tools/semantic_closure.py:758-790`). The proposal's recorded digest
`828fb8678dd9f116da63365df7ee1c814ac09e502c64f7c50f6fba9f9fe59e9c`
therefore correctly binds the current `candidate.diff`; it is not a required
digest of the destination source. The current destination digest is separately
captured by the semantic commit receipt.

Independent full-file source review establishes that the only conditional
directives in `vendor/linux/include/linux/pci_ids.h` are the include guard at
lines 10-11 and its closing `#endif` at line 3270. The guard controls C
preprocessing and has no Rust runtime, linkage, or constant analogue. Every
one of the 2,902 active object-like numeric `#define` macros from lines 15-3268
maps, in source order, to exactly one `pub const NAME: i32 = VALUE;` in the
candidate. All literals are representable as C `int` and Rust `i32`; the
candidate contains only required immutable provenance and those constants.

Accordingly, F001 is disproved, no source edit is warranted, and the copied
final closure records remain unchanged from the sealed proposal.
