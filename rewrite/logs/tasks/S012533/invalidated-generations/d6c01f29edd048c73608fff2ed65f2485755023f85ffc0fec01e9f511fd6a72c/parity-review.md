# Parity review — S012533 (slot 1)

## Scope and evidence

- Reviewed candidate: `src/include/asm-generic/device.rs`.
- Pinned upstream source: `vendor/linux/include/asm-generic/device.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Queue row is `S012533`, `P02`, `REVIEWING`, architecture set `common`.
- The frozen x86_64 and aarch64 metadata each record an architecture-generated
  `asm/device.h` wrapper and a dependency on the same upstream generic header.
  Neither architecture supplies a source `asm/device.h` override.  Upstream
  `include/asm-generic/Kbuild` makes `device.h` mandatory for those targets.

## Comparison

The upstream header has exactly two selected declarations: the empty aggregate
types `struct dev_archdata` and `struct pdev_archdata`, protected only by the
ordinary header guard.  It has no members, conditional fields, functions,
macros (other than its inclusion guard), storage, or architecture-specific
branches.

The candidate has exact immutable provenance for the task, upstream path,
revision, and `common` architecture scope.  It declares both and only both
aggregate types, with no fields and no invented configuration or architecture
members. `#[repr(C)]` correctly expresses their C aggregate/layout role when
embedded by value in `struct device` and `struct platform_device`; the local
Linux device headers show those exact embedding sites.  Public visibility does
not create an upstream data member, symbol, control path, or configuration
delta.

## Result

PASS — no parity findings. The candidate preserves the complete selected
generic definition for both frozen architectures. No compiler, formatter,
rust-analyzer, build, test, or runtime tool was run.
