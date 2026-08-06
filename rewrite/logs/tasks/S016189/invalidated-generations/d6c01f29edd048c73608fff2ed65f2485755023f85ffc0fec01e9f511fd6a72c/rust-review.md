# Rust review — S016189 (slot 2)

Reviewer role: Rust reviewer  
Model / effort: gpt-5.6-terra / high  
Scope: `src/include/uapi/linux/input-event-codes.rs` against pinned
`vendor/linux/include/uapi/linux/input-event-codes.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result: REJECT — ABI/type and UAPI-source contract unresolved

The task provenance is internally consistent: the destination names the pinned
source, revision, common architecture, and task ID (candidate lines 1–5). The
canonical queue verification passed for the frozen fingerprint
`d6c01f29edd048c73608fff2ed65f2485755023f85ffc0fec01e9f511fd6a72c` and the
task was `REVIEWING` on P02 at review start. No compiler, formatter,
rust-analyzer, build, test, or runtime tool was used.

I exhaustively compared the complete header's 795 non-guard `#define` names and
normalized replacement expressions with the 795 Rust constants. Every name,
literal, alias, and the nine `*_CNT` additions matches textually after removing
C/Rust syntax-only whitespace and parentheses. The source has no
function-like macro, shift, mask, or side-effecting macro in this header, so
there is no candidate-specific shift, evaluation-order, or arithmetic-overflow
defect in those expressions. The bounded additions are all within the stated
values (for example `KEY_MAX` is `0x2ff` and `KEY_CNT` is `KEY_MAX + 1`; source
lines 836–838; candidate lines 691–693).

### R1 — all macro constants were fixed to `u32`, changing their required C
conversion/type behavior (blocking)

Every candidate definition imposes `u32` (for example `EV_SYN` at candidate
line 24, `KEY_ESC` at line 45, `KEY_CNT` at line 693, and `ABS_CNT` at line
754). The pinned defines are unsuffixed integer constant expressions and
therefore participate as the C `int` literals/expressions they spell, with the
usual C promotions and context conversion; the pinned file does not declare
them as 32-bit unsigned values (for example source lines 39–63 and 836–838).

This is observable at the UAPI and core boundaries. `input_event.type` and
`.code` are `__u16` (pinned `include/uapi/linux/input.h`:44–46), whereas the
candidate constants are irreducibly `u32`; Rust does not provide C's implicit
integer conversions. Counts are also consumed as array bounds: UAPI
`uinput_user_dev` uses `ABS_CNT` in four `__s32` array declarators (pinned
`include/uapi/linux/uinput.h`:223–230), and core `input_dev` derives its
`unsigned long` bitmap bounds from each `*_CNT` (pinned
`include/linux/input.h`:143–151). A single `u32` binding cannot preserve both
the C literal's contextual conversions and Rust's `usize` array-length
requirement without use-site changes or a defined compatibility mechanism.

The candidate contains neither that mechanism nor a task-scoped ABI decision
establishing that all translated consumers convert at the same point and with
the same width/sign/overflow semantics. This is not a cosmetic annotation:
it changes accepted expressions at every direct `u16`/array-bound consumer.
The applier must establish and implement the frozen Rust representation/export
strategy for code values and count expressions, including all relevant
conversion boundaries; guessing a universal `u32` type is not sufficient.

### R2 — `pub const` does not preserve this header's UAPI preprocessor/DTS
visibility (blocking)

The original is expressly a C *and devicetree-source* header and therefore is
restricted to comments and defines (pinned
`include/uapi/linux/input-event-codes.h`:5–7). It is directly included by the
public UAPI input header (pinned `include/uapi/linux/input.h`:20). Its 795
names are preprocessing-time UAPI names, not linkable data symbols.

The candidate supplies only Rust module `pub const` items (candidate
lines 14–808). They do not yield a C-preprocessor include file or devicetree
definitions, and no source-level UAPI-generation/export boundary is present
in the candidate. The Rust spelling and numeric values are necessary but do
not by themselves preserve this visibility contract. This matters independently
of R1: a C/DTS consumer cannot observe Rust module constants at all.

Before this task can be accepted, the applier must locate the frozen mechanism
that retains or deterministically emits the pinned UAPI include surface for C
and DTS consumers, and record its ownership/ABI evidence. If no such mechanism
exists in the frozen task scope, exact source/ABI parity cannot be established
and the task should be BLOCKED rather than accepted as a Rust-only substitute.

## Manual Rust safety review

There are no pointers, references, unsafe blocks, layout-bearing types,
allocation, `Drop`, concurrency, atomics, shifts, masks, or runtime control
flow in the candidate. Hence no independent aliasing, lifetime, panic, or
unsafe-boundary finding was identified. The rejection is limited to the
typed-constant and UAPI visibility contracts above.
