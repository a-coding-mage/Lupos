# Parity review — S016384 (slot 1)

Reviewed `src/include/uapi/linux/snmp.rs` against the complete pinned
`vendor/linux/include/uapi/linux/snmp.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Finding P1 — UAPI SPDX exception was removed

**Severity:** blocking

The candidate changes the source license identifier from
`GPL-2.0 WITH Linux-syscall-note` to `GPL-2.0-only`.

- Upstream evidence: `include/uapi/linux/snmp.h:1` is
  `/* SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note */`.
- Candidate evidence: `src/include/uapi/linux/snmp.rs:1` is
  `// SPDX-License-Identifier: GPL-2.0-only`.

The syscall-note exception is material UAPI provenance and must be retained;
the changed identifier is not an allowlisted branding delta.

## Exhaustive comparison record

- All eight upstream anonymous `int` enum groups (at upstream lines 19, 69,
  110, 129, 155, 171, 313, and 352) are represented as `c_int` constants.
- The 298 numeric exported values (296 enumerators plus
  `__ICMPMSG_MIB_MAX` and `__ICMP6MSG_MIB_MAX`) match exactly, including every
  ordinal and all eight terminal `__*_MAX` constants. Both macros retain value
  512.
- There are no structs, unions, typedefs, function declarations, conditional
  feature branches, or ABI/linkage declarations in the upstream header.
- The C include guard (`_LINUX_SNMP_H`, upstream lines 8–9 and 375) has no
  public runtime/value counterpart in Rust; Rust module loading supplies the
  one-definition property. No candidate macro/value mismatch arises from that
  language-level transformation.
- Candidate immutable provenance source path, task id, architecture set, and
  Linux revision agree with the queue and `vendor/linux.SHA`.

No other parity defects found. Candidate requires resolution of P1 before it
can be accepted.
