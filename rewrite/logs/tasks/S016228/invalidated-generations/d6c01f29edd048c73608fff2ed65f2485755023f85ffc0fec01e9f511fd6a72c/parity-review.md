# Parity review — S016228

Reviewed source-only on `feat/bun-like-rewrite-test` against pinned
`vendor/linux/include/uapi/linux/lockd_netlink.h` (revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`), the frozen x86_64/aarch64 header
closure, `SYMBOLS.tsv`, `ABI.tsv`, `LIFETIMES.tsv`, and the selected generic
netlink consumers `fs/lockd/netlink.c`, `fs/lockd/netlink.h`, and
`Documentation/netlink/specs/lockd.yaml`. No compiler, formatter, linker,
test, or historical Lupos source was used.

## Findings

1. **P1 — `LOCKD_FAMILY_NAME` changes an object-like macro use into a named
   Rust static and does not supply the stated C array-to-pointer decay.**
   Linux line 10 defines an object-like macro expanding to the string literal
   `"lockd"`: its C type is `char[6]` (with the frozen `-funsigned-char`) and
   it decays at pointer-consuming expressions. In the selected consumer,
   `fs/lockd/netlink.c:38` initializes the generic-netlink family `.name`
   pointer directly with that expansion. Candidate line 14 instead declares
   `pub static LOCKD_FAMILY_NAME: [u8; 6]`. That creates a named static object
   with one fixed identity; using it in a Rust raw-pointer consuming expression
   does not perform C array-to-pointer decay, despite lines 11–13 asserting it
   does. This changes the public macro/API shape and leaves the required pointer
   use to a later non-equivalent adaptation. Represent the macro without
   inventing static storage, and make the consuming generic-netlink translation
   preserve the original literal-to-pointer conversion and trailing NUL.

2. **P2 — the required generated-UAPI provenance notices are omitted.**
   The candidate preserves the exact dual SPDX expression and required rewrite
   provenance, but it drops all four upstream generated-header notices at Linux
   lines 2–5: direct-edit prohibition, YAML source path,
   `YNL-GEN uapi header`, and the exact regeneration command. Candidate line 7
   substitutes a generic description. Retain the original generated-header
   notices (after the immutable rewrite provenance block) so the UAPI's
   generation authority and regeneration route remain auditable.

## Checked parity

- `LOCKD_FAMILY_VERSION` is represented as `c_int = 1`, matching the C integer
  constant in both frozen GNU11 command contexts.
- Both anonymous enum member sequences and their `MAX` expressions have the
  correct C `int` values: server attributes 1, 2, 3, 4, 3; commands 1, 2, 3,
  2. The header declares anonymous enum types only and no enum object/layout;
  the candidate appropriately adds no fabricated layout.
- The exact SPDX dual-license expression, Linux source path, pinned revision,
  architecture membership, and task ID are present. No branding variance was
  found.

Result: **changes required**. The applier must resolve both findings and close
the task's Phase-0 `PENDING_REVIEW` ABI/lifetime records from source evidence
before `DONE`.
