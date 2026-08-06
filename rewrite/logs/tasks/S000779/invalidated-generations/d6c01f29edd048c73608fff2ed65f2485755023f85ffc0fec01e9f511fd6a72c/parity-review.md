# S000779 parity review (slot 1)

Reviewer: parity reviewer, independent source comparison

Reviewed candidate: `src/arch/x86/include/uapi/asm/ldt.rs`  
Pinned source: `vendor/linux/arch/x86/include/uapi/asm/ldt.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`  
Frozen architecture/configuration: x86_64; `CONFIG_X86_64=y`,
`CONFIG_MODIFY_LDT_SYSCALL=y`

## Comparison result

The candidate contains every operative declaration from the complete 48-line
pinned header for the frozen x86_64 branch:

| Pinned declaration | Candidate representation | Result |
| --- | --- | --- |
| `LDT_ENTRIES` = C `int` 8192 | `pub const LDT_ENTRIES: i32 = 8192` | matched |
| `LDT_ENTRY_SIZE` = C `int` 8 | `pub const LDT_ENTRY_SIZE: i32 = 8` | matched |
| `user_desc.entry_number`, `base_addr`, `limit` | three source-order `u32` fields | matched (three x86 `unsigned int` objects) |
| `seg_32bit:1` | bits 0, mask `0x01` | matched |
| `contents:2` | bits 1--2, mask `0x06` | matched |
| `read_exec_only:1` | bit 3, mask `0x08` | matched |
| `limit_in_pages:1` | bit 4, mask `0x10` | matched |
| `seg_not_present:1` | bit 5, mask `0x20` | matched |
| `useable:1` | bit 6, mask `0x40` | matched |
| x86_64-only `lm:1` | bit 7, mask `0x80` | matched |
| `MODIFY_LDT_CONTENTS_{DATA,STACK,CODE}` = 0, 1, 2 | three `i32` constants with 0, 1, 2 | matched |

`#[repr(C)] user_desc` has three consecutive four-byte fields followed by the
four-byte bit-field allocation unit, preserving the required 16-byte object
shape and the 24 unused high bits of that unit.  Each getter has the source
field's mask/shift; each setter truncates its input to the declared field width
and preserves all other bits, including those unused high bits.  This matches
the frozen x86_64 source order/bit allocation used by `fill_ldt`, `LDT_empty`,
`LDT_zero`, and TLS validation in `arch/x86/include/asm/desc.h` and
`arch/x86/kernel/tls.c`.  The `lm` declaration is correctly present only for
the selected x86_64 task; the source's assembler exclusion has no Rust-source
counterpart.

No unauthorized branding, selected-source omission, macro-value mismatch,
or candidate control/ABI representation defect was found.

## Finding P1 — task ABI/lifetime records remain unresolved

`rewrite/ABI.tsv:17330` records `struct user_desc` as `PENDING_REVIEW` for
layout/ABI fields, and `rewrite/LIFETIMES.tsv:17239` leaves its semantic fields
`PENDING_REVIEW`. `rewrite/SYMBOLS.tsv:35305-35317` likewise retains each
selected conditional, macro, and `struct user_desc` record as
`PENDING_REVIEW`. The candidate's correct-looking `repr(C)` encoding is not a
replacement for closing those authoritative task records. Before `DONE`, the
applier must resolve them with this pinned-header/context evidence, explicitly
recording x86_64 `unsigned int` widths, the 16-byte/4-byte-aligned field layout,
the final allocation unit's bit map (0--7 declared; 8--31 retained), the
configuration-conditioned `lm`, and the fact that this passive UAPI data type
has no ownership, lock, RCU, refcount, or cleanup protocol.

Disposition: no source edit requested; this record-closure requirement must be
resolved in the applier's evidence/resolution before a valid `DONE` transition.

No compiler, formatter, build, test, emulator, debugger, or benchmark command
was run.
