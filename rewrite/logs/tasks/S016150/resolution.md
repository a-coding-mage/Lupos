# Resolution — S016150 / attempt 1 / P02

Applier: `gpt-5.6-terra` (high).  This adjudication used only the pinned
source at `425f94c2954b1fe80ebdbf9b29854e89750355df`, frozen Phase 0 records,
the sealed candidate and its evidence, and the two source-only review reports.
No compiler, formatter, test, analyzer, runtime command, or historical Lupos
source was used.

## Dispositions

### P1 — candidate snapshot incomplete: accepted, blocking

`candidate.diff` contains only a prose summary and not the sealed Rust source.
It therefore cannot bind the provenance header or the public `HSR_A_*` and
`HSR_C_*` definitions reviewed in `src/include/uapi/linux/hsr_netlink.rs`.
Replacing it would alter the sealed candidate evidence and require a fresh
implementation/review cycle; it cannot be repaired during this application.

### P2 — anonymous-enum ABI closure: accepted, blocking

The pinned header declares anonymous C enums at lines 21 and 39.  Its direct
consumer uses their enumerators as netlink policy-array indices, generic-netlink
commands, and `maxattr`/`resv_start_op` values
(`vendor/linux/net/hsr/hsr_netlink.c:209-216,544-571`).  That proves ordinal
values and uses, but no frozen ABI record establishes the exact Rust
representation, layout/alignment, or export boundary for either anonymous enum:
`rewrite/ABI.tsv:191348-191349` remains `PENDING_REVIEW`.  Consequently the
candidate's `i32`-constant substitution is not a source-backed closure of
SC1-0a4c9c9b2e80f00f2d2b5a25a857739f03f795ebb816ffa9f8759782b8bf1b94,
SC1-ab2d1d6cffe1147e5b295fac62031b4251be698c5d5a1e4e06acd5d5b54f95e1,
SC1-27fb36b8e071aed5bca22b395196219d98e5f15f85b2d37079141109874089a8,
SC1-58d479125c1d22fb3a9518c4e80cac2b07b3c5c8e52a7472b0eba3989ef00cd8,
SC1-8c69a56f345454402ae823e2943dcc558ad3ec8ed0d1701c54aa6cbad5280c80,
and SC1-09ddd53d99733ff4e1fb4d09fc74144ca90a60ef8c253b7b9846d2d878510c70.
The literal `SOURCE_REVIEWED_VALUE` is not such evidence.  No exact mapping
can be established from the frozen source and records, so this closure remains
unresolved.

### P3 / R1 — C inclusion guard: accepted, blocking

`__UAPI_HSR_NETLINK_H` and its `#ifndef`/`#endif` at pinned lines 14-15 and 51
provide repeatable C textual-inclusion semantics.  The selected records in
`rewrite/SYMBOLS.tsv:366856-366858` remain `PENDING_REVIEW`, and no frozen
module-map or original-driver ABI evidence maps that C-preprocessor boundary to
the Rust module graph.  A Rust constant or an invented macro would not provide
the C behavior.  Thus SC1-fc590a192de2ca1b902b2560abc5338e79fc723d4c9b7e01cdeefda2a73393f6,
SC1-0c4b20a8025addc5dac2efe8154c59fa302802934c5d1ee1a57294600b4dc186,
and SC1-7ba3752fe0d3945832e20610fc82d872438edc35dfaf7f0a23873ea4ab6340b2
cannot be closed from source-only evidence.

### P4 / R2 — copyright and author notice: accepted, blocking in sealed attempt

The pinned source’s 2011-2013 Autronica Fire and Security AS copyright and
Arvid Brodin author attribution at lines 2-11 are absent from the candidate.
The project requires relevant upstream notices to be retained.  Restoring the
notice would change the sealed source after its incomplete snapshot and thus
requires a fresh candidate evidence binding and new independent reviews.  It
is not silently applied here.

## Result

`BLOCKED`.  Exact source-level parity cannot be established for the selected
anonymous-enum ABI and include-guard contract with the frozen records.  The
candidate remains unchanged, and no semantic-closure final commit or `DONE`
transition is authorized.
