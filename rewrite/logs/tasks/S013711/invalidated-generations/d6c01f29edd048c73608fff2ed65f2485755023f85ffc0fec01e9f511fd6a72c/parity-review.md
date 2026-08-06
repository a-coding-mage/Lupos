# Parity review — S013711

Reviewer: parity reviewer (slot 1), P02  
Scope: `include/linux/device-id/i2c.h` → `src/include/linux/device-id/i2c.rs`  
Method: manual pinned-source and frozen-metadata inspection only. No compiler,
formatter, linker, test, emulator, debugger, or rust-analyzer diagnostic was
run.

## Oracle and configuration evidence

- The complete pinned oracle is
  `vendor/linux/include/linux/device-id/i2c.h:1-19` at revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.  Its operative declarations are
  the `__KERNEL__`-guarded `kernel_ulong_t` typedef, `I2C_NAME_SIZE`,
  `I2C_MODULE_PREFIX`, and `struct i2c_device_id`; it defines no function,
  static object, section annotation, or exported data symbol.
- Both frozen command families select `__KERNEL__` and contain
  `-funsigned-char` in their captured commands.  Both targets are LP64.
  Therefore the selected struct has unsigned byte elements, an 8-byte,
  8-aligned `kernel_ulong_t`, `name` at offset 0 for 20 bytes,
  `driver_data` at offset 24 after four bytes of ABI padding, alignment 8, and
  total size 32.
- Direct source consumers make the macros and representation observable:
  `drivers/i2c/i2c-core-base.c:106-119` walks a zero-name terminated ID table
  and compares `id->name` as a C string; lines 176 and 687 compose
  `I2C_MODULE_PREFIX` with a client name; and
  `scripts/mod/file2alias.c:856-861` uses adjacent literal concatenation
  `I2C_MODULE_PREFIX "%s"`.  Selected and driver-owned ID tables use
  string-literal `name` initializers and a final all-zero entry.

## Findings

### P1 — Required immutable provenance SPDX value is wrong (reject)

The candidate begins with `GPL-2.0`.  The fresh-source protocol requires every
translated file to begin with the immutable provenance form
`// SPDX-License-Identifier: GPL-2.0-only`.  The source header's own SPDX is
recorded through the pinned upstream provenance; the candidate must use the
required Rust-file provenance identifier.  This violates the task's required
file header.

Required resolution: replace the first provenance line with the mandated
`GPL-2.0-only` spelling and retain the existing source/revision/architecture/
task lines unchanged.

### P2 — `I2C_MODULE_PREFIX` was changed from a replacement literal into a
typed reference constant (reject)

Oracle line 12 is an object-like macro whose replacement is the literal token
`"i2c:"`.  It declares no header object, pointer/reference value, linkage, or
address identity, and must permit the direct adjacent-literal use in
`file2alias.c`.  The candidate's public `&[u8; 5]` constant instead exposes a
Rust reference value (including reference semantics) under the macro's name.
It cannot expand at each use to a literal and cannot represent
`I2C_MODULE_PREFIX "%s"` under that same name.  Its five bytes do correctly
include the C literal's terminating NUL at a use site, but the changed
expansion/API semantics remain a parity defect.

Required resolution: model this as an invocation-time literal expression
without declaring an addressable header object, pointer/reference alias, or
additional public substitute.  Its standalone expanded C-string bytes must be
exactly `i2c:\0`; translated format consumers must preserve the source
composition so the final format byte sequence is `i2c:%s\0`.

### P3 — `I2C_NAME_SIZE` changed C `int` expression semantics to `usize`
(reject)

Oracle line 11 replaces the macro with the unsuffixed integer literal `20`,
which has C `int` type under both frozen command families.  The candidate
instead publishes `usize`, changing the value's width, signedness, promotions,
and arithmetic behavior outside the one array-bound use.  The resulting array
extent happens to be 20, but that does not preserve the operative macro.

Required resolution: represent the macro as an `int`-typed constant expression
(for example, an invocation-time `20i32` expression), making an explicit
conversion only at the Rust array-bound site.

### P4 — `i2c_device_id` lost ordinary C value-copy behavior (reject)

The oracle declares a plain C struct with no resource-owning member, so C
assignment and by-value initialization copy its complete 32-byte object while
leaving the source usable.  The candidate has neither `Copy` nor `Clone`.
Consequently Rust moves the value on assignment and changes the behavior
available to translated table/initializer and local-value users.  This is not
an ABI-layout issue, but it is a source-level semantic change for this
declaration.

Required resolution: derive `Copy, Clone` on the `#[repr(C)]` struct without
altering its field order, visibility, layout, or zero-table terminator
representation.

## Items that match, subject to the fixes

- `kernel_ulong_t = u64` matches the `__KERNEL__` branch on both frozen LP64
  targets.  It remains a typedef with no storage or linkage.
- `[u8; 20]` is the correct element representation for the source `char[20]`
  because both captured command families use `-funsigned-char`; it is not a
  Rust string and retains the C string initializer/zero-sentinel byte model.
- `#[repr(C)]` and the declared field order give the required 20-byte name,
  four-byte padding, then 8-byte `driver_data` layout.  `driver_data` is an
  opaque driver-private unsigned word; the header supplies no ownership,
  locking, RCU, refcount, allocation, callback, or lifetime protocol.
- `name` is not const-qualified in the oracle, so retaining a mutable public
  field does not itself diverge from this header's field contract.  The header
  defines no ID-table object; source table initializers and their final zero
  entry remain responsibilities of each owning translated or original driver
  unit.

## Pending-record closure required before DONE

The S013711 rows in `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` still state
`PENDING_REVIEW`.  Source evidence resolves the required facts for both
architectures: preprocessing guards have no runtime ABI; the selected
`__KERNEL__` branch supplies an unsigned 64-bit `kernel_ulong_t`; the two
macros are compile-time expressions only; and `i2c_device_id` has the layout
and no independent ownership/lifetime/synchronization behavior stated above.
The applier must record final dispositions in its required resolution workflow;
this review does not edit frozen manifests.

## Verdict

Reject candidate as submitted.  P1--P4 require source-level resolution from
the pinned oracle.  No build, test, or compiler-derived acceptance evidence
was used.
