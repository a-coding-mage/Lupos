# Resolution — S016028 / attempt 1 / P02

## Source evidence reopened

The pinned source is `vendor/linux/include/uapi/asm-generic/termbits-common.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`.  Its lines 2--3 and 66 define
the `__ASM_GENERIC_TERMBITS_COMMON_H` preprocessor guard, lines 9--64 define
the selected replacement-token macros, and line 64 defines the omitted
`TCIOFLUSH` macro as `2`.

The direct UAPI consumer `vendor/linux/include/uapi/asm-generic/termbits.h`
includes this header at line 4 and declares `tcflag_t` as `unsigned int` at
line 6.  Direct pinned TTY code (`vendor/linux/drivers/tty/tty_baudrate.c`)
uses the macros in `tcflag_t` expressions, including the shift/mask operations
at lines 62, 90, 143, 159--160, 184, and 196.

## Disposition of review findings

1. **Parity: omitted `TCIOFLUSH` — upheld.**  The candidate snapshot lacks
   the selected line-64 macro.  Adding a `pub const` alone would not resolve
   the two broader UAPI-contract findings below; any source edit would also
   invalidate the sealed candidate and completed independent Rust review.

2. **Parity: lost include-guard/C-preprocessor interface — upheld.**  The
   candidate contains Rust items only and supplies neither the line-2--3/66
   guard nor a source-proven generated C-header or ABI compatibility boundary.
   The frozen records retain these conditionals and macros as selected
   `PENDING_REVIEW` entries.  No local pinned source or frozen manifest states
   how a Rust destination file must preserve availability to C/preprocessor
   consumers.

3. **Parity and Rust: fixed Rust constant types change the macro-expression
   contract — upheld.**  The C header exposes untyped replacement tokens.  In
   the direct `tcflag_t` uses above they participate in C integer conversions;
   the candidate splits the namespace into `i32` values and one `u32` value
   (`CRTSCTS`).  That cannot reproduce the C expression interface without a
   specified cross-language macro/consumer representation.  In particular,
   preserving the bit pattern does not establish the semantics of complements,
   shifts, or mixed unsigned operations at the cited call sites.

The parity report did not produce a valid semantic-closure attestation or
valid finding-key mapping.  I do not fabricate one.  The valid slot-2
attestation identifies the affected macro expression records, but it likewise
cannot close the missing compatibility-boundary decision.

## Outcome

`BLOCKED`.  Exact source parity requires a project-level, source-proven
representation for selected UAPI C macro and preprocessor contracts, including
their availability to C consumers and their contextual integer conversion
behavior.  The pinned source, current candidate, and frozen guidance do not
establish that mapping.  No candidate source or semantic-closure final record
was changed during this adjudication.
