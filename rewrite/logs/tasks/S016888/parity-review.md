# S016888 parity review — slot 1

Status: FINDINGS

## P1-LOCK_EVENT-MACRO-CONTRACT

Semantic records: `SC1-719fb53c6e968aeb5dba2192fd55f3a04b1d06e5bccb7ab39bd87b3c98ace361`, `SC1-71cc1816c82055647a12d1ca208b27cf9666b1434ca1a38d8f6fdded9d10e97f`.

Linux symbol: `LOCK_EVENT`.

Pinned local evidence: `kernel/locking/lock_events_list.h:16-18` supplies a default one-argument expansion, `LOCKEVENT_ ## name,`, only when the includer has not already defined `LOCK_EVENT`; every selected list item is then a separate `LOCK_EVENT(name)` expansion (lines 44-102). `kernel/locking/lock_events.h:19-25` relies on that default form to produce the `enum lock_events` members. `kernel/locking/lock_events.c:26-28,46-50` deliberately redefines the same one-argument macro and reincludes the list to produce one designated string initializer per event.

Candidate evidence: `src/kernel/locking/lock_events_list.rs:14-56` instead exports `lock_events_list!` and invokes its supplied macro once with the complete comma-separated identifier list. This requires a different variadic callback interface, supplies no equivalent default `LOCKEVENT_`-identifier expansion, and cannot preserve either direct consumer contract above. The event sequence itself is correct for the frozen union, but the operative macro behavior and its public identifier contract are not.

Required resolution: retain the per-event, one-identifier expansion mechanism (including the default enum-name generation and consumer override capability) for the frozen selected event sequence; do not replace it with one aggregate callback invocation.
