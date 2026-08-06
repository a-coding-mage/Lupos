# S016888 implementation

Translated `kernel/locking/lock_events_list.h` to
`src/kernel/locking/lock_events_list.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source is an X-macro event list.  For both frozen configurations,
`CONFIG_QUEUED_SPINLOCKS=y` and `CONFIG_PARAVIRT_SPINLOCKS` is absent.  The
translation therefore preserves the selected 38-entry order as C `int`-repr
`lock_events` discriminants and as the ordered spelling list needed by the
other X-macro consumer.  The excluded PV-only entries are not emitted.

No build, formatter, test, or runtime command was run.
