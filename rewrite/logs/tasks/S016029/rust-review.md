# Rust review — S016029

## Result

PASS — no Rust ownership, layout, integer-conversion, public-surface, or safety defect found in `src/include/uapi/asm-generic/termbits.rs`.

## Evidence reviewed

- Pinned source: `vendor/linux/include/uapi/asm-generic/termbits.h` at revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, including all 149 lines.
- Dependency S016028: `src/include/uapi/asm-generic/termbits-common.rs`, which exposes `cc_t = u8` and `speed_t = u32` from the included UAPI header.
- Frozen task/scope records: S016029 is the `common` translation of this header for both approved targets and has only S016028 as a dependency; S016028 is DONE.

## ABI and Rust-semantics audit

- `tcflag_t = u32` is the selected-target representation of C `unsigned int`; the re-exported `cc_t = u8` and `speed_t = u32` preserve C `unsigned char` and `unsigned int`.  The public aliases and structure-tag spellings are retained.
- Each record has `#[repr(C)]` and preserves source member order.  With the source aliases, `termios` has the four 32-bit flags at offsets 0, 4, 8, and 12, `c_line` at 16, and `c_cc[19]` at 17; it is size 36 and alignment 4.  `termios2` and `ktermios` additionally place `c_ispeed` and `c_ospeed` at offsets 36 and 40, respectively; each is size 44 and alignment 4.  No explicit padding or packing is missing.
- `NCCS` is an `i32`, matching the C unsuffixed integer literal category, and its only Rust-specific conversion is the required const array-length conversion.  Every other value macro in this source header is an unsuffixed literal representable as C `int` on both frozen targets and is represented as `i32`; the dependent header separately retains its unsigned `CRTSCTS` literal as `u32` where C literal typing requires it.
- A normalized source-to-candidate comparison of every source value macro (including `NCCS`) found no missing, extra, or changed name/value pair.  The C include guard has no Rust value analogue and is correctly represented by module inclusion.  The source contains no configuration conditional other than that guard.
- The candidate contains no `unsafe`, `unsafe fn`, `Drop`, borrowing/aliasing mechanism, allocation, panic/unwrap/expect path, test configuration, placeholder, or executable behavior.  `Copy` on these plain C-layout scalar records does not add ownership or drop semantics absent from C.

No source changes were made by this reviewer.  No compiler, formatter, linker, test, or runtime command was run.
