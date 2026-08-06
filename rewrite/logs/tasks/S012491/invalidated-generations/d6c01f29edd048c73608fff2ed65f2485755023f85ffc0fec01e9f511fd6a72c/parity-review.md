# Parity review — S012491

## Result: reject; blocker requiring applier disposition

The candidate is a comment-only description and does not preserve the only
operative behavior of `include/acpi/platform/acgccex.h`: in each C translation
unit that reaches this header, lines 20–22 test whether the preprocessor macro
identifier `strchr` is defined and, only in that case, remove that exact macro
from the remainder of that translation unit.  This is not a declaration of a
`strchr` function or a runtime value.

`src/include/acpi/platform/acgccex.rs` declares no item or mechanism at all.
Rust module parsing and Rust macro resolution have no API that can observe or
mutate the C preprocessor macro environment of a C translation unit, so the
module cannot perform the upstream `#ifdef strchr` / `#undef strchr` action.
Nor can a Rust module emulate the C include guard `__ACGCCEX_H__`, whose scope
is the preprocessing state of an individual C translation unit.  A prose
assertion that Rust has no such namespace is not a source-level preservation
of those selected preprocessor branches.

This is materially selected in both frozen configurations.  `SYMBOLS.tsv`
rows 130411–130420 inventory the guard and the `strchr` conditional for both
`aarch64` and `x86_64`, and leave their final semantic status
`PENDING_REVIEW`.  `header_include_edges.tsv` shows the path
`acpi/acpi.h -> acpi/platform/acenvex.h -> acgccex.h` under the Linux
`__GNUC__` branch (at `acenvex.h:39`) for the selected AArch64 Xen and x86 ACPI
boot command contexts.  `header_closure.tsv` records 2,304 AArch64 and 510
x86_64 consumers, spanning `LINUX_DRIVER_OBJECT,RUST_TRANSLATE`; it therefore
cannot be treated as an unused Rust-only source comment.

The frozen configurations enable `CONFIG_ACPI=y` on both architectures (and
`CONFIG_XEN=y` on AArch64), and the recorded compile contexts target
`aarch64-linux-gnu` and `x86_64-linux-gnu` with the GNU-compatible compiler
environment.  The frozen evidence does not provide a per-translation-unit
preprocessor-state result for whether `strchr` is defined at this inclusion
point.  A source-only reviewer must not invent that state or run a compiler to
obtain it.  More importantly, even either result cannot be reproduced by the
candidate Rust module: C driver-object consumers must retain the original C
header preprocessing, while Rust consumers have no C macro namespace for this
module to mutate.

The candidate does retain the upstream dual SPDX expression and the Intel
copyright year/range, plus required immutable provenance.  Those retained
comments do not cure the operative omission.  There are no function, layout,
or ABI entries for this header in `ABI.tsv` or ownership records in
`LIFETIMES.tsv`; the unsolved contract is instead the selected macro and
translation-unit environment.  Under the frozen one-to-one `RUST_TRANSLATE`
mapping, exact parity cannot be established with a Rust source file alone.

Required applier disposition: either provide frozen, source-level authority
for a boundary that preserves the original header for every C consumer and
proves the Rust module is never required to model this effect, while closing
each listed conditional record; or mark S012491 `BLOCKED`.  Do not accept this
empty module as a faithful implementation merely because it exports no
runtime symbol.

## Sources inspected

- `vendor/linux/include/acpi/platform/acgccex.h` (complete header)
- `src/include/acpi/platform/acgccex.rs` (complete candidate)
- `vendor/linux/include/acpi/platform/acenvex.h`, `acpi/acpi.h`, and
  `include/linux/acpi.h`
- Frozen `SCOPE.tsv`, `FILE_MAP.tsv`, `SYMBOLS.tsv`, `ABI.tsv`,
  `LIFETIMES.tsv`, both frozen configurations, and header-closure/include-edge
  metadata

No compiler, formatter, test runner, rust-analyzer diagnostic, or historical
source was used.
