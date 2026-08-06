# Rust review — S013681

Reviewed source-only as independent Rust reviewer (slot 2) for P01.

## Scope verified

- Branch: `feat/bun-like-rewrite-test`.
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Queue row: `S013681` is `REVIEWING`, maps
  `include/linux/decompress/unlzma.h` to
  `src/include/linux/decompress/unlzma.rs`, and covers `common`.
- The selected header inventory contains only its include-guard conditionals
  and `DECOMPRESS_UNLZMA_H` macro (both x86_64 and aarch64):
  `rewrite/SYMBOLS.tsv` rows for `S013681`.

## Finding R1 — `char *` callback parameter has the wrong frozen C character type

**Severity: major.**

`unlzma`'s final callback parameter is declared as `void (*error)(char *x)`
in the pinned header at `vendor/linux/include/linux/decompress/unlzma.h:10`.
The candidate instead declares it as
`unsafe extern "C" fn(*mut core::ffi::c_char)` at
`src/include/linux/decompress/unlzma.rs:30`.

For both approved architectures, the frozen original compile commands carry
`-funsigned-char`: `rewrite/FILE_MAP.tsv:16459` (aarch64) and
`rewrite/FILE_MAP.tsv:21462` (x86_64).  Thus this source file's C `char` has
the unsigned-character semantics fixed by the Phase 0 invocation, whereas
Rust's target `c_char` follows the target's native C ABI type and does not
encode this per-command `-funsigned-char` override.  Exposing the callback
argument as `*mut c_char` therefore gives Rust callback implementations a
different pointee signedness contract from the selected Linux declaration.

The implementation retains this exact callback type in `struct rc` and calls
it with error strings (`vendor/linux/lib/decompress_unlzma.c:80`, `:98`,
`:557`, `:583`, `:613`, and `:628`), so this is an operative cross-language
callback contract, not an unused spelling difference.  `generic.h` likewise
uses the same callback in the `decompress_fn` interface
(`vendor/linux/include/linux/decompress/generic.h:5-11`).

Resolution required: represent this frozen `char *` parameter as an unsigned
byte pointer (for example `*mut c_uchar`) in the callback function-pointer
type, while retaining the existing `unsafe extern "C"` ABI and non-null
function-pointer representation.  This preserves the raw-pointer lifetime and
provenance model—no Rust reference or stronger lifetime guarantee should be
introduced.

## Other Rust/FFI observations

- `Option<unsafe extern "C" fn(*mut c_void, c_ulong) -> c_long>` correctly
  represents the two nullable C function pointers; the Linux implementation
  tests each callback before selecting/calling it
  (`vendor/linux/lib/decompress_unlzma.c:105-109`, `:657`).
- Raw mutable pointers for `buf`, `output`, and `posp` preserve the header's
  nullable, aliasable C pointer contracts.  The implementation explicitly
  tests these at `vendor/linux/lib/decompress_unlzma.c:555`, `:601`, and
  `:655`; no borrowed Rust reference is appropriate.
- `c_int`, `c_long`, `c_ulong`, and `c_uchar` otherwise faithfully preserve
  the declared scalar and pointer widths for the common LP64 targets.  No
  layout-bearing types, allocations, `unsafe` bodies, or Rust ownership/drop
  mechanisms are introduced by this header declaration.

No compiler, formatter, rust-analyzer, build, test, debugger, or runtime tool
was used.  No source or queue file was modified.
