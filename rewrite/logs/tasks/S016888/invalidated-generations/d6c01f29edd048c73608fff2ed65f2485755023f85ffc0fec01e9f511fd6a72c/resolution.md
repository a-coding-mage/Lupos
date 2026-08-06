# S016888 resolution

Applier reopened pinned `vendor/linux/kernel/locking/lock_events_list.h`,
`kernel/locking/lock_events.h`, and `kernel/locking/lock_events.c` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, together with both frozen
configurations. No build, formatter, test, or runtime command was run.

## Dispositions

1. **Parity P1 / Rust RUST-1:** accepted. The candidate incorrectly turned
   the `LOCK_EVENT` X-macro fragment into specific products. The destination
   now exposes only `lock_event_list!`, a continuation macro which passes the
   selected ordered names to a caller-provided consumer macro. Its caller owns
   the expansion, matching the source's default-versus-overridden `LOCK_EVENT`
   inclusion role.
2. **Parity P1 / Rust RUST-2:** accepted. Removed the list-task-owned
   `lock_events` enum. `kernel/locking/lock_events.h` owns the enum and its
   two trailing equal-valued enumerators, `lockevent_num` and
   `LOCKEVENT_reset_cnts`; that dependent task remains the sole owner of those
   artifacts.
3. **Parity P1 / Rust RUST-3:** accepted. Removed `LOCK_EVENT_NAMES`. The
   unselected `kernel/locking/lock_events.c` owns its private C names table
   and its `.reset_counts` designated initializer, so no Rust table belongs in
   this header-list translation.
4. **Rust RUST-4:** accepted. Restored the source SPDX identifier exactly to
   `GPL-2.0`.

Both frozen configurations select `CONFIG_QUEUED_SPINLOCKS` and do not select
`CONFIG_PARAVIRT_SPINLOCKS`; the reusable list therefore retains the 38
non-PV entries in exact source order. This task owns no ABI object, allocation,
lock, reference, RCU, or unsafe boundary. The configuration-selected list
contract is complete; enum and table ownership are explicitly delegated to
their source-owning consumers.
