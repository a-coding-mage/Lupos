# Parity review — S016028 (slot 1)

## Verdict

ACCEPT — no parity findings.

## Evidence reviewed

- Pinned source: `vendor/linux/include/uapi/asm-generic/termbits-common.h`
  at `425f94c2954b1fe80ebdbf9b29854e89750355df`, lines 1–66.
- Candidate: `src/include/uapi/asm-generic/termbits-common.rs`, lines 1–66.
- Frozen task/scope records: `S016028` is the path-preserving common
  `RUST_TRANSLATE` header task for both frozen architectures.  The symbol
  inventory has, for each architecture, two guard conditionals, the include
  guard macro plus 45 functional macros, and the two typedefs.
- Header-closure evidence records 155 AArch64 and 84 x86_64 selected
  consumers.  Direct selected uses include `net/nfc/nci/uart.c` and
  `net/bluetooth/rfcomm/tty.c`; their `CRTSCTS` operations are on
  `ktermios::c_cflag` / `tcflag_t`.

## Exhaustive comparison

- Provenance is exact: the candidate retains the UAPI SPDX expression and
  records the exact pinned source path, revision, common architecture scope,
  and task ID.  There is no branding delta.
- `cc_t` maps `unsigned char` to `u8`; `speed_t` maps target `unsigned int`
  to `u32`.  The downstream generic termbits UAPI header defines `tcflag_t`
  as `unsigned int`, so these mappings preserve the relevant 8-bit and
  32-bit ABI widths.  This file declares no aggregate, packed, aligned, or
  FFI layout.
- All 45 functional macros are present with their original names and exact
  values: the ten `c_iflag` bits, six `c_oflag` bits, sixteen base baud
  selectors, `EXTA`/`EXTB` aliases, `ADDRB`, `CMSPAR`, `CRTSCTS`, `IBSHIFT`,
  the four `tcflow`/`TCXONC` selectors, and the three `tcflush`/`TCFLSH`
  selectors.  `EXTA` and `EXTB` remain aliases of `B19200` and `B38400`.
  Hex-digit separator insertion does not change any value or bit position.
- Literal-category check: all source numeric literals except `CRTSCTS`
  (`0x80000000`, an unsigned-int hex constant on both target ABIs) are
  representable as C `int`; the two aliases retain their base values.  The
  Rust candidate intentionally expresses every operative value as `u32`.
  That is the exact width and signedness of the consuming `tcflag_t` flag
  words and matches the usual C arithmetic conversion at each observed flag
  use; `CRTSCTS` is consequently represented exactly.  The action/flush
  selectors and `IBSHIFT` retain their non-negative values.  No signedness,
  mask, alias, or shift-value discrepancy results.
- The only source conditionals are the conventional include guard and its
  closing `#endif`; Rust module inclusion supplies the one-definition
  behavior, so omitting a Rust constant for the C-private guard is correct.
  There are no feature, sparse, Kconfig, or architecture-specific branches
  in this header.
- The candidate contains no functions, mutable/static state, allocation,
  control flow, unsafe code, tests, placeholders, or substitute mechanism.

No source, manifest, or queue file was edited by this reviewer; no build,
formatting, test, or runtime command was run.
