# Parity review — S014172 (slot 1)

## Scope and evidence

Reviewed the complete pinned `vendor/linux/include/linux/kern_levels.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/linux/kern_levels.rs`, the task's symbol inventory, and relevant
pinned uses in `vendor/linux/include/linux/printk.h` (for example,
`printk(KERN_INFO pr_fmt(fmt), ...)` and `printk(KERN_CONT fmt, ...)`).  No
configuration conditional occurs in the source header.

## Findings

### P1 — string-like macros no longer have C literal / C-string semantics

`KERN_SOH`, `KERN_EMERG` through `KERN_DEBUG`, `KERN_DEFAULT`, and `KERN_CONT`
are C preprocessor macros whose expansions are string-literal tokens.  The
pinned definitions at `include/linux/kern_levels.h:5,8-17,24` consequently:

1. concatenate with adjacent literals/tokens at the caller at translation
   time (for example `KERN_INFO pr_fmt(fmt)` in `include/linux/printk.h`);
2. denote NUL-terminated `char` arrays when used as C strings, including the
   trailing terminator; and
3. decay to C character pointers rather than becoming Rust fat `&str` values.

The candidate's `pub const ...: &str` definitions at
`src/include/linux/kern_levels.rs:8,13-23,30` retain only the non-NUL payload
bytes.  They cannot participate in the source's adjacent-token concatenation
and do not supply the required trailing NUL or pointer representation to a
C-facing consumer.  This is an observable change to the macro and C-string
contract, not merely a Rust spelling difference.

The applier must replace these `&str` exports with an exact Rust mechanism
that preserves the required compile-time composition and C-string storage/
termination semantics at translated call sites, with any ABI-facing view
explicitly documented.  Do not retain `&str` as the authoritative equivalent.

### P2 — SPDX identifier differs from the pinned source

The pinned source begins `/* SPDX-License-Identifier: GPL-2.0 */`, while the
candidate begins `// SPDX-License-Identifier: GPL-2.0-only`.  `GPL-2.0-only`
matches the generic provenance example in `AGENTS.md`, but it is not the SPDX
identifier retained from this source as required by the same file's source-tree
rules.  The applier must resolve this protocol conflict explicitly rather than
leave the source-specific identifier silently changed.

## Verified mappings

- `KERN_SOH_ASCII` has the source character literal's value `1` and a C `int`
  representation (`core::ffi::c_int`).
- `LOGLEVEL_SCHED` through `LOGLEVEL_DEBUG` preserve every source integer value
  and use `core::ffi::c_int`, consistent with the source's unsuffixed integer
  constants on the approved Linux targets.
- The spelled non-NUL payload bytes for every log-prefix level match the source
  expansion after `KERN_SOH` substitution.
- The Linux-source path, revision, task ID, and `common` architecture
  provenance fields match the task row and `vendor/linux.SHA`.

## Result

REJECT pending disposition of P1 and P2.  No source, build, formatting, or
test action was performed by this reviewer.
