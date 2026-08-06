# Implementation — S014373

Implemented `src/include/linux/migrate_mode.rs` from the complete pinned oracle
`vendor/linux/include/linux/migrate_mode.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Scope and lease checked before editing: common x86_64/aarch64 header task,
attempt 1, pipeline `P01`, worker `codex-root-20260806-p01`. Phase 0 identity
and queue verification were successful (queue fingerprint
`af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`).

The Linux header contains only the include guard, documentation, and two C
enums. The Rust file represents both selected enum types as public `#[repr(C)]`
enums, retaining declaration order, explicit Linux enumerator spellings, and
therefore C's implicit consecutive discriminants beginning at zero.

Relevant source context was inspected in `include/linux/migrate.h` and the
migration, compaction, memory-policy, memory-failure, memory-hotplug, and DAMON
call sites. Those consumers distinguish these values by identity/order; this
translation introduces no replacement behavior, storage, or control flow.

No compiler, formatter, test, runtime command, historical Rust source, or
non-leased source file was used or changed.
