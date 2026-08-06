# S016888 parity review (slot 1)

Reviewed `src/kernel/locking/lock_events_list.rs` against pinned
`vendor/linux/kernel/locking/lock_events_list.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its including header
`kernel/locking/lock_events.h`, and the frozen x86_64/AArch64 configurations.
No build, formatter, test, or runtime command was run.

## Result: reject; applier changes required

### P1 — `lock_events` is not the enum expanded by the Linux source

`lock_events_list.h` is an X-macro fragment, not the definition of
`enum lock_events`.  The actual enum is formed in `lock_events.h:19-25` by
including this list and then appending `lockevent_num` and
`LOCKEVENT_reset_cnts = lockevent_num`.  The candidate declares a standalone
Rust `lock_events` enum containing only the 38 list entries.  It consequently
omits both final C enumerators (each with value 38), despite the source comment
claiming to retain the C enum order.  It also precludes the translation of
`lock_events.h` (S016887) from constructing the complete enum without
duplicating or conflicting with this declaration.

The applier must preserve the list as reusable list/expansion semantics, or
otherwise make the full `lock_events.h` enum (including the two trailing
enumerators and their equal value) the single canonical definition.  Do not
present the list-only sequence as the complete C enum.

### P1 — the custom `LOCK_EVENT` expansion is not preserved; the added name
### list is an incomplete, unselected substitute

At `lock_events_list.h:16-18`, `LOCK_EVENT` has a default only when an
including context has not supplied it.  In particular,
`kernel/locking/lock_events.c:26-50` undefines/redefines it to produce
designated string-table initializers, then includes this header again.  The
candidate exposes neither a reusable X-macro/list expansion nor an equivalent
mechanism for that context.  `LOCK_EVENT_NAMES` is not that expansion: the
Linux string table has `lockevent_num + 1` entries and explicitly adds
`[LOCKEVENT_reset_cnts] = ".reset_counts"`; the Rust array has only the 38
list entries and cannot encode the source's designated-initializer semantics.

Moreover, both frozen configurations have `CONFIG_LOCK_EVENT_COUNTS` unset,
so `kernel/locking/Makefile:40` does not select `lock_events.o`; a partial
stand-in for its string table must not be added as if it were selected
behavior.  Remove this substitute unless a complete selected consumer
requires an exact representation, and retain the context-dependent list
semantics needed by the mapped headers.

## Verified facts

- Provenance matches `vendor/linux.SHA` and the task/source/architecture
  mapping: S016888, `kernel/locking/lock_events_list.h`, `common`.
- Both frozen configurations set `CONFIG_QUEUED_SPINLOCKS=y`.
- x86_64 explicitly has `CONFIG_PARAVIRT_SPINLOCKS` unset; AArch64 has no
  such selected config symbol/definition.  Thus the 11 PV-only entries at
  upstream lines 25-35 are correctly excluded for the frozen union.
- The remaining selected list has exactly 38 entries.  The candidate's 38
  listed event spellings, order, and discriminants 0 through 37 match the
  default `LOCK_EVENT(name) -> LOCKEVENT_ ## name,` expansion.  The failure is
  its treatment as a complete enum and generic name-list replacement, not the
  selected event ordering or PV exclusion.
