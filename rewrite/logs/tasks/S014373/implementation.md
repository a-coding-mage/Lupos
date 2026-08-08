Task S014373 is BLOCKED before destination-source creation.

Provenance checked:

- branch: `feat/bun-like-rewrite-test`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- source: `vendor/linux/include/linux/migrate_mode.h`
- destination: `src/include/linux/migrate_mode.rs`
- architectures: `common` (selected by both frozen x86_64 and AArch64 configurations)
- lease: P02, owner `codex-root-p02`, attempt 1

The complete pinned header contains the `MIGRATE_MODE_H_INCLUDED` include guard,
the explanatory comments, `enum migrate_mode` with values
`MIGRATE_ASYNC = 0`, `MIGRATE_SYNC_LIGHT = 1`, `MIGRATE_SYNC = 2`, and
`enum migrate_reason` with values `MR_COMPACTION = 0` through `MR_DAMON = 9`
and sentinel `MR_TYPES = 10`. No conditional branch changes these definitions.

Direct pinned-source uses were inspected in migration/compaction, page-owner,
trace, and internal memory-management callers. They use the enums as C enum
parameters/fields and the reason values as an indexed domain; no local source
evidence establishes a Rust-compatible underlying integer width or signedness.

`rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` retain `PENDING_REVIEW` for both
enum types on both architectures. The frozen task evidence therefore does not
establish the enum ABI/layout needed to choose between Rust representations
(including `repr(C)`), and choosing one would violate the zero-difference and
no-guessing requirements. No destination file or placeholder was created.

Required unblock: authoritative frozen ABI/compiler evidence for the underlying
C enum representation on x86_64 and AArch64, followed by semantic-record
closure and a resumed implementation attempt.
