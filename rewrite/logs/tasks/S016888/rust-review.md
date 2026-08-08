# Rust source review — S016888 (slot 2)

Review status: **FINDINGS**

## RUST-1 — blocking: the Rust macro changes the X-macro expansion interface

Semantic keys: `SC1-1c8054fbe8344160a4916147666381994fe0608781b51fdd924862b45f37ed73`, `SC1-719fb53c6e968aeb5dba2192fd55f3a04b1d06e5bccb7ab39bd87b3c98ace361`, `SC1-71cc1816c82055647a12d1ca208b27cf9666b1434ca1a38d8f6fdded9d10e97f`, `SC1-168314c67271d6b24e7c6c1873545c60e2e5629019155ad360bc12691a0b335d`, `SC1-2ae9141303721547bfa3beaf8aeec445588eef365eb97ee72c292bfc216f632e`.

Pinned `kernel/locking/lock_events_list.h:16-17` establishes a default object-like consumer contract: when no consumer definition is active, each `LOCK_EVENT(name)` expands separately to `LOCKEVENT_ ## name,`.  The list then invokes that macro separately for every selected item, in order.  A consumer may replace `LOCK_EVENT` before including the list; `kernel/locking/lock_events.h:16-22` relies on the default to emit one enum identifier per invocation, and `kernel/locking/lock_events.c:22,46-50` redefines it as `[LOCKEVENT_ ## name] = #name,` so the same repeated invocations emit designated string-table initializers.

`lock_events_list!` instead accepts an `ident` and performs exactly one expansion, `$lock_event! { lock_pending, ... }`.  This is a different token interface and lexical expansion shape: it neither invokes the supplied transformation once per event nor supplies the upstream default `LOCKEVENT_` token-pasted identifier output.  A Rust consumer corresponding to either pinned use must now implement a distinct list-accepting macro and cannot receive the per-item tokens and punctuation that the header exports.  In particular, the upstream name-table transformation needs token pasting and stringification for each item; that behavior is absent rather than preserved.

This has no `unsafe` block to audit, but it is nevertheless a source-level Rust macro/lexical-context defect.  Replace the interface with a faithful, usable per-entry expansion mechanism (or block the task if Rust macro facilities cannot express the selected C contract) and update the semantic closure only with evidence for that mechanism.
