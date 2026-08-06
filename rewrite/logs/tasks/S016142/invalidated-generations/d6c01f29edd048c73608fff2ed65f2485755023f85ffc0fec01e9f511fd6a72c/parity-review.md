# Parity review — S016142 (slot 1)

## Verdict

PASS — no parity findings.

## Evidence reviewed

- Pinned source: `vendor/linux/include/uapi/linux/handshake.h`, revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/uapi/linux/handshake.rs`.
- Frozen x86_64 and AArch64 commands for the direct in-tree consumer
  `net/handshake/genl.c`; neither carries `-fshort-enums`, so the three
  ordinary C enum tags retain the normal 32-bit C `int` representation used
  by the candidate's transparent `c_int` newtypes.
- Source consumer: `net/handshake/genl.c` uses `HANDSHAKE_FAMILY_NAME` and
  `HANDSHAKE_FAMILY_VERSION` for the generic-netlink family and uses the
  command and attribute constants as the generated UAPI wire identifiers.

## Complete declaration comparison

- SPDX is retained exactly as
  `((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`; provenance identifies
  the correct Linux path, pinned revision, common architecture scope, and task.
- The source declares no struct, union, bitfield, function, conditional
  configuration branch, or layout-bearing UAPI object.  Thus there is no
  struct layout, alignment, packing, or field ABI to preserve in this file.
- The three named enum tags are present as distinct transparent `c_int`
  wrappers: `handshake_handler_class` (0, 1, 2), `handshake_msg_type`
  (0, 1, 2), and `handshake_auth` (0, 1, 2, 3).  Every source enumerator and
  its ordinal is represented exactly, including the handler-class terminal
  sentinel.
- Every anonymous-enum UAPI value has the source `int` representation and
  exact sequence: X509 attributes 1..3 with max 2; ACCEPT attributes 1..10
  with max 9; DONE attributes 1..4 with max 3; and commands 1..4 with max 3.
  The candidate expresses each public `*_MAX` as the same terminal-minus-one
  computation, rather than substituting a divergent value.
- `HANDSHAKE_FAMILY_VERSION` is retained as `c_int` value 1.  The three C
  string-literal macros retain their exact ASCII bytes and terminating NUL in
  immutable `c_char` static arrays: `"handshake"` (10 bytes), `"none"`
  (5 bytes), and `"tlshd"` (6 bytes).  These provide the same static
  string storage and pointer-decay use through `.as_ptr()`.
- The C include guard has no Rust-language ABI counterpart; there is no
  omitted operative declaration behind it.  No unauthorized branding, test
  configuration, placeholder, or executable behavior was introduced.

No source was built, formatted, or executed during this review.
