# S013730 parity review (slot 1)

Outcome: **REJECT — source changes required.**  This was a manual source-only
comparison; no compiler, formatter, linker, test, or historical Lupos source
was used.

Reviewed authority:

- `vendor/linux/include/linux/device-id/rpmsg.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df` (complete 19-line source);
- frozen x86_64 and AArch64 configurations (`CONFIG_64BIT=y` on both);
- task `S013730` entries in `SCOPE.tsv`, `SYMBOLS.tsv`, `ABI.tsv`, and
  `LIFETIMES.tsv`;
- direct consumer evidence in `vendor/linux/drivers/rpmsg/rpmsg_core.c`,
  `vendor/linux/drivers/rpmsg/rpmsg_char.c`, and selected
  `vendor/linux/net/qrtr/smd.c`.

## Findings

1. **P1 — `RPMSG_NAME_SIZE` no longer has the C macro's expression type.**
   Linux line 11 defines the object-like macro as the unsuffixed literal `32`,
   whose C type is `int`.  The candidate instead exports `pub const
   RPMSG_NAME_SIZE: usize = 32`.  This changes any future expression using the
   name from C `int` arithmetic/promotions to Rust `usize`, rather than only
   converting to a Rust array bound at the point where that is required.
   `rpmsg_core.c` uses the macro as a `strncmp`/`memcpy` count and array index,
   and the header is selected for both frozen targets.  Preserve the
   object-like expansion (for example, an `i32` macro invocation) and perform
   an explicit `as usize` solely in the Rust array bound.

2. **P1 — `RPMSG_DEVICE_MODALIAS_FMT` was changed from an object-like C
   string-literal macro into a reference-valued Rust constant.**  Linux line
   12 expands at each use to the NUL-terminated literal `"rpmsg:%s"`; it does
   not declare a header object or pointer alias.  The candidate's `&[u8; 9]`
   creates a reference-valued alias with different expansion/address behavior.
   The difference is operative: `rpmsg_core.c` relies on literal concatenation
   in `RPMSG_DEVICE_MODALIAS_FMT "\\n"` and in
   `"MODALIAS=" RPMSG_DEVICE_MODALIAS_FMT`.  Translate it as an object-like
   macro whose invocation expands to `b"rpmsg:%s\\0"`, not as a `&[u8; 9]`
   constant.

3. **P2 — `struct rpmsg_device_id` is missing C aggregate-copy semantics.**
   C permits copying this record by value; its direct consumers declare static
   ID tables, including the aggregate tables in `rpmsg_char.c` and `smd.c`.
   The candidate has `#[repr(C)]` and the correct field order/width for the
   two LP64 frozen targets, but it does not derive `Copy, Clone` like the
   corresponding translated device-ID records.  Add `#[derive(Copy, Clone)]`
   while retaining `#[repr(C)]`, so Rust-side table/aggregate handling can
   preserve the C record's bitwise-copy behavior without a bespoke move-only
   semantic.

## Confirmed portions

- `kernel_ulong_t = u64` matches C `unsigned long` under both frozen 64-bit
  configurations.
- The 32-byte `name` field followed by that 8-byte member gives the required
  LP64 C field ordering and 40-byte record layout when kept `#[repr(C)]`.
- The fixed byte array correctly avoids substituting a Rust string.
- Immutable task/source/revision provenance is present and the Linux revision
  matches `vendor/linux.SHA`.
