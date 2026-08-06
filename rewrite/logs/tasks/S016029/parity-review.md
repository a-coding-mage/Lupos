# S016029 parity review (slot 1)

Reviewed `vendor/linux/include/uapi/asm-generic/termbits.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/asm-generic/termbits.rs` for the frozen `common` scope
(x86_64 and aarch64).

## Result

No parity findings.

## Evidence checked

- The Rust provenance and SPDX identifier name the exact scoped Linux header,
  revision, architecture scope, and task ID.  No branding delta, test code,
  configuration conditional, or out-of-scope declaration was added.
- The source has one dependency, `asm-generic/termbits-common.h`; the candidate
  imports its completed Rust counterpart's `cc_t = u8` and `speed_t = u32`.
  `tcflag_t` is correctly `u32` for C `unsigned int`.
- `termios`, `termios2`, and `ktermios` each carry `#[repr(C)]`, retain every
  member in upstream order, and use `[cc_t; 19]` for `c_cc`.  With the selected
  aliases this preserves the two-target UAPI record layouts: 36 bytes for
  `termios`, and 44 bytes each for `termios2` and `ktermios`; their four-byte
  member alignment and tail placement also match C.  `Clone, Copy` does not
  affect those layouts.
- The direct textual inventory agrees: all 97 source declarations excluding
  the C include guard are present in the candidate (`NCCS` plus 96 value
  macros), with no extra constants.  Each literal value is unchanged,
  including the aliases `XTABS == TAB3`, `CBAUDEX == BOTHER`, and the complete
  B57600--B4000000 range.  Every source literal fits C's signed `int` on both
  frozen targets, so the candidate's `i32` constant types preserve the C
  literal type; no macro expression or configuration branch is omitted.
- The source contains no `termio` or `winsize` definition; their absence from
  this one-to-one translation is therefore not an omission.  The relevant
  `termios2` ioctl consumers require its C-size/layout, which the candidate
  retains.

No compiler, formatter, test, linker, runtime, or other validation command was
run.
