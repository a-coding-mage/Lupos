# S016189 resolution — attempt 1 / P01

Manual source application only. No compiler, formatter, linker, test, runtime,
or rust-analyzer diagnostic was used.

Pinned source: `vendor/linux/include/uapi/linux/input-event-codes.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` (1,016 lines). Final destination:
`src/include/uapi/linux/input-event-codes.rs`, SHA-256
`acba0f9a93f2c8efb4544a192eb71c8b9544efe4c5521f1c7171ac17c74821d2`.

## Dispositions

1. `PARITY-001` — **DISPROVED.** The Rust destination does not replace the
   pinned C/DTS header. The frozen map assigns this task only the Rust path,
   while `rewrite/metadata/header_closure.tsv:6939,11257` records C consumers
   of `vendor/linux/include/uapi/linux/input-event-codes.h`. That unchanged
   pinned header retains all `#define` tokens and its preprocessor interface;
   no C/DTS emission path exists in this file task.
2. `PARITY-002` — **RESOLVED_CHANGED.** All 795 non-guard definitions are now
   `i32` declarations, matching the source's unsuffixed integer constants,
   aliases, and derived `+ 1` expressions (`input-event-codes.h:23-1014`).
3. `PARITY-003` — **RESOLVED_CHANGED.** Removed the invented numeric guard
   declaration. Upstream uses an empty replacement list at lines 16–17 and
   closes it at line 1016; Rust module loading is the corresponding
   single-definition behavior, with no exported numeric value.
4. `RUST-S016189-001` — **DISPROVED.** `semantic_closure.py:531-560` defines
   the semantic candidate artifact as `candidate.diff`. Its current hash is
   `9937906354485fd12dbeef6173d96d49e1d544e951445d366311ed3125980181`, the
   exact sealed-proposal `candidate_sha256`; the review compared that binding
   incorrectly to the destination file hash.
5. `RUST-S016189-002` — **RESOLVED_CHANGED.** The fabricated public guard
   item is gone; see upstream lines 16–17 and final Rust lines 21–22.
6. `RUST-S016189-003` — **RESOLVED_CHANGED.** The complete 795-definition
   Rust representation uses `i32`; `INPUT_PROP_CNT` and `KEY_CNT` retain their
   source expressions and signed source type (`input-event-codes.h:33,838`).

## Final source checks

- The source has 795 non-guard `#define` names; the destination has 795 `i32`
  declarations, with no missing or extra names.
- No `u32` declarations or `_UAPI_INPUT_EVENT_CODES_H` value remain.
- The three continued source comments are valid Rust comments adjacent to the
  same definitions; no comment continuation consumes following declarations.
- No placeholder, Rust test configuration, unsafe, layout, ABI, locking, or
  driver mechanism is present in this constants-only header.

Semantic final/disposition evidence carries the same six dispositions and
source citations for the reviewed closure records.
