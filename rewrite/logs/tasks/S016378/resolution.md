# S016378 application resolution — attempt 2

## Result

BLOCKED.  The frozen one-to-one mapping cannot preserve this selected UAPI
C-header interface at `src/include/uapi/linux/serial_reg.rs`.  No source change
is safe within the leased destination: a Rust source file is not a C
preprocessor include surface.  The blocker is therefore recorded rather than
replacing the source interface with a Rust-only approximation.

## Evidence reopened by the applier

- Pinned source and provenance: `vendor/linux.SHA` and the candidate both name
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The complete pinned `include/uapi/linux/serial_reg.h` is selected for
  `common`; its include guard is at lines 15-16 and 386.  It defines every
  operative interface as a C `#define`, including the function-like
  `UART_FCR_R_TRIG_BITS(x)` at lines 100-103.
- The frozen scope row maps that source only to
  `src/include/uapi/linux/serial_reg.rs`, with 63 aarch64 and 26 x86_64 header
  consumers.  The symbol inventory records the guard and operative macros for
  both architectures, including `UART_FCR_R_TRIG_BITS`, as `PENDING_REVIEW`.
- `vendor/linux/include/linux/serial.h:13` directly includes
  `<uapi/linux/serial_reg.h>`.  The selected 8250 context makes the macro
  contract operative: `struct serial8250_config::fcr` is `unsigned char` in
  `drivers/tty/serial/8250/8250.h:67-73`, and
  `8250_port.c:2978` evaluates
  `UART_FCR_R_TRIG_BITS(up->fcr)`.  The same byte register state is modified
  with `~UART_FCR_TRIGGER_MASK` at `8250_port.c:2661`.

## Finding dispositions

### P1 — all UAPI macro definitions lose their C preprocessor surface

Confirmed; unresolved within frozen scope.  The source contract is an include
guard plus 231 C preprocessor macro definitions, whereas the frozen destination
is solely a Rust file.  `pub const` and `macro_rules!` do not create C
preprocessor definitions for the C includee established above.  Neither the
frozen scope, file map, ABI/lifetime records, nor porting guidance supplies a
C-compatible companion-header/generation/interface mechanism that could be
used without expanding the task or introducing a new unreviewed design.

Disposition: BLOCKED pending a reviewed Phase-0 scope/interface decision that
can preserve the C/assembly-facing header contract alongside any Rust mapping.

### P1 — `UART_FCR_R_TRIG_BITS` is not semantically or lexically equivalent

Confirmed; unresolved within frozen scope.  Pinned lines 100-103 define the
function-like macro exactly as:

`(((x) & UART_FCR_TRIGGER_MASK) >> UART_FCR_R_TRIG_SHIFT)`

It is ordered between `UART_FCR_R_TRIG_SHIFT` and
`UART_FCR_R_TRIG_MAX_STATE`, expands in the C preprocessor, evaluates its
argument once, and applies C's usual integral promotions to the demonstrated
`unsigned char` `up->fcr` caller.  The candidate's crate-root `macro_rules!`
item is not that C preprocessor interface, and Rust does not supply C's
implicit byte-to-`int` promotion through a fixed `i32` macro expression.
Changing it to a fixed `i32` function, a comment, or an approximate Rust macro
would therefore weaken the pinned contract.  No permitted source evidence
defines a cross-language interface that retains the macro expansion and caller
promotion behavior.

Disposition: BLOCKED pending the same reviewed interface decision; no
unreviewed Rust-only replacement was applied.

### P2 — required source order and substantive comments are not retained

Confirmed.  The candidate displaces the `DLAB=0` and `DLAB=1` section comments,
moves `UART_FCR_R_TRIG_BITS` after `UART_FCR_R_TRIG_MAX_STATE`, and abbreviates
the pinned trigger table and later register/RSA/OMAP/ALTR commentary.  The
pinned source requires the original ordering and substantive text.  Restoring
comments/order alone would still leave the operative C macro surface absent,
and presenting that incomplete Rust shell as a completed translation would be
misleading.

Disposition: BLOCKED with the interface blocker above; this finding remains
confirmed and must be corrected as part of any reviewed full interface
translation.

### RUST-S016378-01 — function-like macro loses C integer promotion

Confirmed; same root blocker as the parity P1 macro finding.  The local source
consumer proves the operand is byte-sized, and its source expansion is governed
by C integer promotion before `& 0xC0` and `>> 6`.  The current Rust macro
instead combines an arbitrary Rust expression with `i32` constants; it neither
accepts the C preprocessor caller nor preserves that promotion rule.  A
byte-only helper would be a different public interface, while a generic Rust
helper cannot restore a C macro definition to C/assembly consumers.

Disposition: BLOCKED pending a reviewed C-facing interface mechanism; no
fixed-`i32` helper or caller-side conversion was introduced.

## Final disposition

All four findings were independently checked against the complete pinned
header and the demonstrated selected caller context.  The required C
preprocessor/public macro interface cannot be established from the frozen
Rust-only mapping and allowed local evidence.  Per the Phase 1 rule, the task
is blocked rather than guessed or weakened.  No compiler, formatter, linker,
test, rust-analyzer diagnostic, or runtime command was used.
