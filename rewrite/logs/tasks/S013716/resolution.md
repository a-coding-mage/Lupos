# S013716 applier resolution

## Source and task identity

- Task `S013716` is the frozen common translation of
  `include/linux/device-id/isapnp.h` to
  `src/include/linux/device-id/isapnp.rs`.
- The required branch is `feat/bun-like-rewrite-test`, and both
  `vendor/linux.SHA` and candidate provenance name
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- `rewrite/SCOPE.tsv:13717` and
  `rewrite/metadata/header_closure.tsv:4564,9563` select the header for both
  frozen targets.  The recorded target commands in
  `rewrite/FILE_MAP.tsv:16494,21493` define `__KERNEL__`; therefore the
  original `kernel_ulong_t` declaration is selected on both targets.

## Review dispositions

Both independent reports contain no findings.  I reopened the complete
pinned header and its immediate Linux use context, including
`include/linux/isapnp.h` and `drivers/pnp/isapnp/compat.c`, and independently
confirm their conclusions:

1. **`kernel_ulong_t` — accepted.**  The selected original declaration at
   `vendor/linux/include/linux/device-id/isapnp.h:5-7` is `unsigned long`.
   `core::ffi::c_ulong` in the candidate is the corresponding C ABI type.
   The frozen x86_64 and AArch64 target headers each set
   `__BITS_PER_LONG` to 64 (`arch/x86/include/uapi/asm/bitsperlong.h:5-8` and
   `arch/arm64/include/uapi/asm/bitsperlong.h:20-25`), so this payload is the
   same 64-bit unsigned-long ABI field on both selected targets.
2. **`ISAPNP_ANY_ID` — accepted.**  The sole operative macro in the source is
   the unsuffixed integer literal `0xffff` at `isapnp.h:9`.  The candidate
   retains its value and frozen-target `int` type as
   `core::ffi::c_int = 0xffff`.  Its kernel consumer passes the macro as the
   two `unsigned short` parameters of `pnp_convert_id`
   (`drivers/pnp/isapnp/compat.c:28-31`); this conversion is unchanged by the
   value-and-type-preserving Rust declaration.
3. **`struct isapnp_device_id` — accepted.**  Lines 10-14 of the original
   contain exactly four consecutive `unsigned short` members,
   `card_vendor`, `card_device`, `vendor`, and `function`, followed by
   `kernel_ulong_t driver_data`.  The candidate's `#[repr(C)]` record retains
   every field, type width, source order, natural alignment, and consequent
   inter-field/trailing padding.  The source declares no arrays, flexible
   arrays, bitfields, pointers, ownership transfer, aliases, or callbacks;
   therefore it introduces no array or pointer-provenance contract beyond
   this by-value C-layout record.  `Copy, Clone` creates no destructor or
   layout change and matches ordinary C structure-value copying.
4. **Conditional/provenance records — accepted.**  The source include guard
   is preprocessing-only, while the selected `__KERNEL__` branch is accounted
   for above.  Candidate SPDX and immutable source/revision/architecture/task
   provenance exactly identify this frozen task.  The empty branding allowlist
   authorizes no deltas, and none was made.

## Final semantic closure

The `PENDING_REVIEW` records for the selected typedef and record are closed by
the above upstream evidence: there is no owned allocation, reference,
pointer provenance, locking, RCU, refcount, callback, or destruction behavior;
the ABI is the `#[repr(C)]` declaration described in disposition 3.  No source
change is required after either review, and no compiler, formatter,
rust-analyzer, build, test, debugger, or runtime command was used.
