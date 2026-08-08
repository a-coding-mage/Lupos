+# Rust source review — S016252, attempt 2

Reviewer: rust_p02_s016252  
Role: independent Rust semantics reviewer (slot 2)  
Scope inspected: pinned `vendor/linux/include/uapi/linux/mptcp_pm.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`, current `src/include/uapi/linux/mptcp_pm.rs`, current candidate summary, and the S016252 rows in the frozen scope/symbol/lifetime/ABI records. No compiler, formatter, test, runtime tool, Git/history, or prior review evidence was used.

## Result: FINDINGS

### RUST-001 — selected named enum type `enum mptcp_event_type` is omitted

- **Pinned evidence:** `include/uapi/linux/mptcp_pm.h:44-56` declares the selected named C type `enum mptcp_event_type`; it is separately inventoried as a `type` for both architectures in `SYMBOLS.tsv`, `LIFETIMES.tsv`, and `ABI.tsv`.
- **Candidate evidence:** the Rust file exports only `MPTCP_EVENT_*` `i32` constants and no public `mptcp_event_type` type.
- **Risk:** C code can name this type in declarations, storage, and interfaces. Constants alone do not preserve that type-level contract or provide a Rust declaration usable by dependent translations. The ABI record is still `PENDING_REVIEW`, so silently choosing the constants' type does not close the required type layout/alignment contract.
- **Required resolution / proposal key:** `add-mptcp-event-type-alias` — add an explicit public representation for the named C enum, with the frozen x86_64/AArch64 enum ABI (including size/alignment) established in the task ABI record. Do not use a restrictive Rust enum for externally supplied netlink values unless it preserves arbitrary C enum bit patterns.

### RUST-002 — selected named enum type `enum mptcp_event_attr` is omitted

- **Pinned evidence:** `include/uapi/linux/mptcp_pm.h:110-132` declares `enum mptcp_event_attr`; frozen symbols/lifetimes/ABI records each select it for x86_64 and AArch64.
- **Candidate evidence:** only `MPTCP_ATTR_*` `i32` constants are present; no `mptcp_event_attr` declaration is exported.
- **Risk:** This repeats the missing named-type/ABI surface. Netlink-derived values are not constrained to listed discriminants, so an idiomatic closed Rust enum could add invalid-value and conversion behavior absent from C.
- **Required resolution / proposal key:** `add-mptcp-event-attr-alias` — expose the named C enum representation after recording its target ABI, preserving arbitrary incoming integer values and avoiding allocation, panics, or changed validation.

### RUST-003 — `MPTCP_PM_NAME` changes C string-literal representation

- **Pinned evidence:** `include/uapi/linux/mptcp_pm.h:10` defines `MPTCP_PM_NAME` as the C string literal `"mptcp_pm"`. It is an operative macro selected for both architectures.
- **Candidate evidence:** `pub const MPTCP_PM_NAME: &str = "mptcp_pm";`.
- **Risk:** a C string literal denotes a NUL-terminated array (and normally decays to a character pointer in expression context); Rust `&str` is a fat, length-carrying reference to eight non-NUL bytes. It cannot preserve callers that require the terminated byte sequence or C-pointer semantics, and it exposes a different ABI/provenance contract.
- **Required resolution / proposal key:** `preserve-mptcp-pm-name-c-string` — provide the NUL-terminated byte representation needed by the C macro contract, and record/offer any pointer-facing form only with a valid static lifetime and exact FFI representation. Retain an ergonomic Rust view only as an additional, non-substituting interface.

## Other manual checks

All listed integer discriminants and sentinel-minus-one values match the pinned header numerically and use `i32`, which is consistent with C integer-constant expressions subject to closure of the enum ABI finding above. This file has no callbacks, storage, pointer arithmetic, `unsafe`, allocation, ownership transfer, Drop timing, pinning, concurrency, endian conversion, bitfields, FFI functions, or recoverable error paths. The anonymous C enum declarations are unnameable and only supply their constants; no additional standalone Rust type is required for them from this header alone. No approval is possible until the three findings are resolved.

