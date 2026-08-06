# S016029 applier resolution

Task `S016029` translates `include/uapi/asm-generic/termbits.h` to
`src/include/uapi/asm-generic/termbits.rs` for the frozen common scope
(x86_64 and aarch64), at pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Independent source recheck

I reopened all 149 lines of the pinned header and the required included source
`include/uapi/asm-generic/termbits-common.h`, then checked the Rust candidate,
the frozen task/scope records, and direct UAPI consumers. The source has the
include guard, one `tcflag_t` alias, three records, `NCCS`, and 96 other value
macros. The candidate has exactly the corresponding public Rust surface:
`tcflag_t = u32`; the dependency's `cc_t = u8` and `speed_t = u32`; all three
`#[repr(C)]` records in source member order; and all 97 value constants.

The direct definition inventory matches name-for-name and value-for-value. All
this header's unsuffixed integer literals, including `CIBAUD`, fit the C `int`
range on both frozen targets, so their `i32` Rust representation preserves the
source literal category. The C guard intentionally has no Rust value item.

`termios` has offsets `0,4,8,12,16,17` and size/alignment `36/4`.
`termios2` and `ktermios` additionally place `c_ispeed` and `c_ospeed` at
`36` and `40`, for size/alignment `44/4`. `termios_internal.h:37-46` uses the
records at UAPI conversion boundaries, and `asm-generic/ioctls.h:61-64` derives
`termios2` ioctl sizes from the same source layout. No source correction is
required.

## Review dispositions

1. Parity review: no findings. Confirmed independently; the complete macro
   inventory, C field order, aliases, and UAPI-only scope are retained.
2. Rust review: no findings. Confirmed independently; `#[repr(C)]`, scalar
   widths, array extent, layout, and absence of unsafe/Drop/ownership machinery
   are appropriate for these caller-owned UAPI values.

## Semantic records closed

- `SYMBOLS.tsv`: all 208 S016029 rows are `COMPLETE`, including both frozen
  architecture instances of the guard, macros, and four type records. Macro
  literal categories and the no-public-item guard treatment are explicit.
- `ABI.tsv`: all eight type rows are `COMPLETE`; each records the selected
  `unsigned int`/`unsigned char` widths and the exact C-layout offsets, sizes,
  and alignment.
- `LIFETIMES.tsv`: all eight type rows are `COMPLETE`; aliases carry no object
  lifetime, while record instances remain caller/user-owned raw UAPI storage
  with no internal locking, RCU, refcount, allocation, callback, or Drop
  family.
- There are no function, static, export, allocation, locking, RCU, refcount,
  or asynchronous rows for this declaration-only header; that no-row family is
  explicitly not applicable.

All five required evidence files now exist. No compiler, formatter, linker,
test, emulator, debugger, runtime command, or benchmark was run.
