# S016378 parity review — slot 1, attempt 2

## Scope and source identity

- Reviewed only pinned `vendor/linux/include/uapi/linux/serial_reg.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df` and the current candidate
  `src/include/uapi/linux/serial_reg.rs`.
- The frozen queue row is `S016378`, `REVIEWING`, `common`, mapping the above
  Linux header to the candidate.  The worktree HEAD reference is
  `refs/heads/feat/bun-like-rewrite-test`; `vendor/linux.SHA`,
  `rewrite/PHASE0_IDENTITY.tsv`, and the candidate provenance all name the
  same revision.
- No compiler, formatter, test, linker, rust-analyzer diagnostic, or runtime
  command was used.  No implementation/review/resolution/history artifact was
  inspected.

## Findings

### P1 — all UAPI macro definitions lose their required C preprocessor surface

Linux symbols: `_LINUX_SERIAL_REG_H`, and all 231 operative `#define` symbols
from `UART_RX` through `UART_ALTR_TX_LOW`.

Local evidence: the pinned UAPI header provides its include guard at lines
15–16 and 386, and defines all 231 operative interfaces with `#define`.
Candidate lines 20–272 replace 230 object-like macros with Rust `pub const`
items, and lines 77–80 replace the remaining function-like macro with a Rust
macro.  Neither representation is a C preprocessor definition available to a
C/assembly includee.  This is not merely a linkage-name issue: the source is a
UAPI header, and `vendor/linux/include/linux/serial.h:13` includes
`<uapi/linux/serial_reg.h>` for C consumers.  The frozen scope row records 63
aarch64 and 26 x86_64 consumers.  No C-compatible header or equivalent
preprocessor surface is present in this leased candidate.

Required resolution: preserve each source macro (including the include-guard
behavior) for C/assembly-facing consumers while providing any Rust-facing
mapping without replacing that surface.

### P1 — `UART_FCR_R_TRIG_BITS` is not semantically or lexically equivalent

Linux symbol: `UART_FCR_R_TRIG_BITS`.

Local evidence: pinned source lines 100–103 defines the function-like C macro
as `(((x) & UART_FCR_TRIGGER_MASK) >> UART_FCR_R_TRIG_SHIFT)`, in scope between
`UART_FCR_R_TRIG_SHIFT` and `UART_FCR_R_TRIG_MAX_STATE`.  Candidate lines
75–80 place it after `UART_FCR_R_TRIG_MAX_STATE` and implement it as
`#[macro_export] macro_rules!`, whose body combines `$x` with `i32` constants.
`#[macro_export]` exposes this macro at the Rust crate root, rather than with
the candidate module's constants; it is neither a C preprocessor macro nor
the source header's include-local macro surface.

The changed operand semantics are material.  The selected local caller
`vendor/linux/drivers/tty/serial/8250/8250_port.c:2978` applies the source
macro to `up->fcr`; `vendor/linux/include/linux/serial_8250.h:132` declares
that member `unsigned char`.  C performs the usual integral promotions before
the source mask/shift.  Candidate line 79 instead forces the operation through
the fixed `i32` constants, so it does not retain the source macro's C operand
and promotion semantics for the demonstrated `unsigned char` input (or its
preprocessor availability).  The source macro must be preserved exactly at the
UAPI boundary; a Rust helper, if needed, must separately model the required
input and result types.

### P2 — required source order and substantive comments are not retained

Linux symbols: `UART_RX`/`UART_TX`, `UART_FCR_R_TRIG_*`,
`UART_FCR_R_TRIG_BITS`, `UART_DLL`/`UART_DLM`, `UART_TRG`, `UART_ACR_*`,
`UART_RSA_*`, and the OMAP/ALTR groups.

Local evidence: source lines 18–22 place the `DLAB=0` section before
`UART_RX`/`UART_TX`; candidate line 22 places its shortened comment after both
constants.  Source lines 59–73 retain the full chip-specific RX/TX trigger
table; candidate line 52 reduces it to one sentence.  Source lines 163–168
place the `DLAB=1` section before `UART_DLL`/`UART_DLM`; candidate line 128
places it between those two definitions.  Source lines 179–205, 231–276,
290–337, and 347–384 retain the register-access, direction, manufacturer,
and per-register commentary; candidate lines 131–239 and 245–272 omit or
materially shorten it.  The source macro order at lines 100–103 also differs
from candidate lines 75–80 as described above.

The task requires source ordering and comments to be checked, and the project
requires relevant upstream notices to be retained.  Restore the source section
ordering and substantive comments verbatim (including the trigger mapping and
RSA attribution/context) unless a frozen manifest explicitly permits a delta;
the branding allowlist contains no relevant entry.

## Passes

- Manual inventory comparison found exactly 231 operative source macros
  (excluding `_LINUX_SERIAL_REG_H`) and exactly 231 candidate declarations
  (230 constants plus `UART_FCR_R_TRIG_BITS`).  Every operative macro name is
  represented once; the only ordinal displacement is
  `UART_FCR_R_TRIG_BITS`/`UART_FCR_R_TRIG_MAX_STATE`.
- The numeric values, duplicate aliases, masks, bit shifts, and derived
  expressions of all 230 object-like symbols match the pinned source's
  evaluated integer values.  This includes the duplicate `0x20` IIR/MCR
  aliases, BRK/ANY_DELTA masks, RSA offsets and flags, `SERIAL_RSA_BAUD_BASE`
  division, and all three `OMAP1_UART*_BASE` values.  Candidate correctly uses
  `u32` for the three `0xfffb....` OMAP constants and `i32` for the remaining
  source-`int` expressions; this pass does not cure the P1 preprocessor/API
  loss.
- Candidate lines 1–5 retain the exact SPDX identifier and required immutable
  source/revision/architecture/task provenance.  It contains no Rust test
  configuration, test item, `todo!`, `unimplemented!`, fake-success path, or
  unauthorized Lupos branding.

## Disposition

REJECT pending resolution of both P1 findings and the P2 source-order/comment
loss.  No source change was made by this reviewer.
