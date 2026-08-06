# Implementation — S014944

Translated `include/linux/sem_types.h` to `src/include/linux/sem_types.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

Both frozen configurations (`rewrite/configs/x86_64/frozen.config` and `rewrite/configs/aarch64/frozen.config`) set `CONFIG_SYSVIPC=y`.  The selected `struct sysv_sem` therefore has exactly one member, `undo_list`, represented as a mutable pointer to the forward-declared opaque `sem_undo_list` type.  The pointee is defined by `ipc/sem.c`; this header neither owns nor exposes its layout.  `sysv_sem` uses `#[repr(C)]` so the enclosing `task_struct.sysvsem` member retains the C field representation.

The source has no functions, storage, cleanup, locking, or executable control flow.  The ownership/lifetime and ABI records require final `PENDING_REVIEW` closure by the later reviewers and applier; no unproven pointee layout was introduced here.  No branding delta applies.
