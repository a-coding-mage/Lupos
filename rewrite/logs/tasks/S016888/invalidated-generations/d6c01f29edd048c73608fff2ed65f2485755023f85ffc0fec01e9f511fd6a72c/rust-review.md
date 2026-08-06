# S016888 Rust review (slot 2)

Verdict: **REJECT**. The selected event names and their 0--37 order are
correct, but the candidate changes the meaning and ownership of this
header-only X-macro list.

Reviewed inputs:

- `vendor/linux/kernel/locking/lock_events_list.h`
- `vendor/linux/kernel/locking/lock_events.h:19-25`
- `vendor/linux/kernel/locking/lock_events.c:26-50`
- `src/kernel/locking/lock_events_list.rs`
- both frozen configurations

## Findings

### RUST-1 — critical — the `LOCK_EVENT` X-macro contract is absent

`lock_events_list.h:16-18` defines `LOCK_EVENT(name)` only when its consumer
has not supplied one; every subsequent entry invokes that macro. This makes
the file a reusable, configuration-selected event-list interface, rather than
an enum or a string-table definition. In the pinned source, the default
expansion is consumed by `lock_events.h:19-25` to form the beginning of
`enum lock_events`, while `lock_events.c:26-50` overrides it to form
designated string-table initializers.

The candidate at lines 14-98 contains neither an equivalent list/callback
interface nor the default-versus-overridden expansion behavior. It instead
materializes two particular consumer products. Consequently the dependent
`S016887` translation cannot use this file to create the full enum and its
trailing `lockevent_num`/`LOCKEVENT_reset_cnts` entries, and any other
selected consumer has no way to obtain the same list expansion. Preserve a
configuration-selected reusable event-list mechanism in this task; consumers
must own their own enum or table output.

### RUST-2 — critical — enum is defined in the wrong source and is incomplete

`lock_events_list.h` declares no enum. The actual `enum lock_events` belongs
to `lock_events.h`, which follows the inclusion with `lockevent_num` and
`LOCKEVENT_reset_cnts = lockevent_num`. The candidate instead exports a new
`pub enum lock_events` containing only the 38 list entries. It lacks both
final enumerators (which have value 38 in the frozen configurations), places
the type in the wrong task/source boundary, and will conflict with the
dependent header's required definition. `#[repr(i32)]` cannot repair this
source/semantic mismatch.

### RUST-3 — high — exported `LOCK_EVENT_NAMES` is not the C object or ABI

The list header defines no names object. `lock_events.c` owns a private
`static const char * const lockevent_names[lockevent_num + 1]`, initializes it
through the overridden X-macro, and supplies the additional `.reset_counts`
entry at `LOCKEVENT_reset_cnts`. The candidate instead exposes
`pub const LOCK_EVENT_NAMES: [&str; 38]`. A Rust `&str` is a fat
pointer-plus-length, not a C `const char *`; the value also has different
visibility, ownership, name, extent, and lacks the reset entry. It must not
stand in for that C-source-owned table.

### RUST-4 — medium — SPDX identifier was changed

The pinned source tag is `SPDX-License-Identifier: GPL-2.0`; the candidate
uses `GPL-2.0-only`. The provenance rule requires retaining the upstream SPDX
identifier, so it must be restored exactly.

## Confirmed facts

`CONFIG_QUEUED_SPINLOCKS=y` for both frozen configurations. The x86_64
configuration explicitly disables `CONFIG_PARAVIRT_SPINLOCKS`; it is absent
from the AArch64 configuration, so the PV block is not selected by either
configuration. The candidate's non-PV 38 spellings and their declaration order
match the selected C expansions. No `unsafe`, layout, pointer, or ownership
construct exists independently of the rejected macro/interface replacement.

No build, formatter, test, or runtime command was run.
