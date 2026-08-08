# S016888 implementation

Translated `kernel/locking/lock_events_list.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to the leased destination.

The source is an X-macro list, not counter storage or lock acquisition logic.
Its Rust macro accepts one callback macro and supplies the selected event names
in upstream order.  The direct consumers in `kernel/locking/lock_events.h` and
`kernel/locking/lock_events.c` respectively generate the event enum and its
debugfs name table from this list.  Both frozen configurations select
`CONFIG_QUEUED_SPINLOCKS`; neither selects `CONFIG_PARAVIRT_SPINLOCKS`, so the
six queued-spinlock entries are present and the eleven PV-only entries are not
in the configuration union.

No allocation, locking, unsafe operation, ABI declaration, counter update, or
runtime behavior is introduced by this list-only translation.
