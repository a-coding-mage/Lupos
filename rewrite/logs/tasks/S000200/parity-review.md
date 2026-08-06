# S000200 parity review (slot 1)

Reviewed candidate `src/arch/arm64/include/asm/vncr_mapping.rs` against pinned
`vendor/linux/arch/arm64/include/asm/vncr_mapping.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` (source SHA-256
`87882faa68e0cea46ad6a2e1cc1fa2d03a470b52a2f96e8cf864cab0c48ce3fd`).

## Finding P1 — SPDX identifier was changed

- **Source evidence:** `vncr_mapping.h:1` says
  `SPDX-License-Identifier: GPL-2.0`.
- **Candidate evidence:** `vncr_mapping.rs:1` says
  `SPDX-License-Identifier: GPL-2.0-only`.
- **Required resolution:** retain the source SPDX identifier exactly as
  `GPL-2.0`.  The rewrite rule requires retaining SPDX identifiers; this
  header has no separate copyright notice to carry over.

## Checked parity

- The source has 104 unconditional object-like `VNCR_*` macros (lines 10–113)
  and the candidate has exactly 104 same-named public constants.  A
  name/value comparison found no missing, extra, or unequal entry.  Every
  displacement is retained exactly, including the non-contiguous regions and
  all 8-byte ICH/MPAM sequences; lowercase hexadecimal spelling is
  semantically immaterial.
- Every C literal fits in a signed 32-bit `int`; the candidate's `i32` values
  have the same width, signedness, and numeric value.  The source expands to
  integer constant expressions; Rust `const` values remain compile-time
  integer values, introduce no mutable state, allocation, or linker object,
  and have no stable address.  There is no pointer/address-taking contract in
  this header.
- The source has no configuration conditional around any mapping.  Its only
  preprocessor conditional is the include guard, which has no corresponding
  runtime or ABI object in a Rust module.  The queue, scope, and provenance
  correctly bind this task to `aarch64` only.
- Consumer evidence in `arch/arm64/include/asm/kvm_host.h:447–451` uses
  `VNCR_ ## r` as byte displacements divided by 8 in enum constant
  expressions.  The candidate preserves the required byte values and signed
  32-bit integer representation for a direct Rust translation of that
  consumer.
- No runtime state, synchronization, layout, linkage, configuration branch,
  exported C ABI symbol, branding delta, or unsafe operation exists in the
  source mapping header.

**Verdict:** reject pending correction of P1; the 104-value mapping itself is
otherwise source-parity complete.  No build, formatter, test, runtime, or
compiler command was run.
