# S016189 implementation

- Lease: `P01`, attempt 1; source: `include/uapi/linux/input-event-codes.h`; destination: `src/include/uapi/linux/input-event-codes.rs`.
- Verified the required branch, queue fingerprint (`af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`), frozen Linux revision (`425f94c2954b1fe80ebdbf9b29854e89750355df`), common architecture membership, and active P01 lease before editing.
- Read the complete 1,016-line pinned UAPI header. It is standalone and contains only the include guard, comments, and object-like event-code defines; it has no selected conditional configuration branches, types, functions, function-like macros, or external header dependencies.
- Translated every non-guard define into one public `u16` constant. `u16` is the exact value domain for this header's non-negative event type/code values (maximum `0x2ff`) and the UAPI event type/code fields; aliases and derived count expressions retain their original identifier dependencies and arithmetic.
- Preserved the upstream SPDX expression, copyright notice, immutable provenance, all define names, numeric spellings, aliases, and derived expressions. Documentation-only comments attached to a define were omitted when necessary to turn the C preprocessor form into valid Rust syntax.
- Mechanical source checks: 795 source non-guard defines and 795 Rust constants; ordered name comparison matched; normalized ordered name/value/expression comparison matched exactly (795/795). Searched the candidate for prohibited placeholders/tests/panic helpers and found none.
- No compiler, formatter, linker, test, runtime, or benchmark command was run.
