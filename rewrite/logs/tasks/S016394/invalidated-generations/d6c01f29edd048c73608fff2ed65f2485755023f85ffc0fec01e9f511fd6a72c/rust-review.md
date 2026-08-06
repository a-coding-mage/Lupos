# Rust review — S016394

## Result

Accepted: no Rust-specific correctness finding.

## Evidence reviewed

- Pinned source: `vendor/linux/include/uapi/linux/sunrpc/debug.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, lines 10–49.
- Candidate: `src/include/uapi/linux/sunrpc/debug.rs`.
- Selected records for both frozen architectures in `rewrite/SYMBOLS.tsv` and
  the `common` scope/map entry for S016394.
- Consumer context: `vendor/linux/include/linux/sunrpc/debug.h` and
  `vendor/linux/net/sunrpc/sysctl.c`; the masks participate in expressions
  with `unsigned int` debug flags after C's usual arithmetic conversions.

## Audit

- The thirteen unsuffixed hexadecimal macro literals are all representable as
  C `int` on both frozen targets. The candidate exposes the same names and
  values as `i32`, preserving the macro expression type before consumer-side
  integer conversions. `RPCDBG_ALL` remains exactly `0x7fff`; no mask bit was
  widened, dropped, or made unsigned prematurely.
- Each anonymous-enum enumerator has C type `int`; its explicit initial value
  and the seven implicit increments are represented exactly by the eight
  `i32` constants (1 through 8). The C declaration names no enum object or
  enum tag, so it creates no ABI-bearing aggregate requiring `#[repr(C)]`.
- The include guard has no Rust item-level or runtime analogue. The frozen
  configurations disabling `CONFIG_SUNRPC_DEBUG` do not conditionally remove
  this UAPI header's masks or enum constants.
- The candidate has exact task/source/revision/architecture provenance, retains
  the UAPI SPDX identifier and upstream copyright notice, introduces no
  branding delta, and contains no `unsafe`, FFI layout, allocation, panic,
  test configuration, placeholder, or executable control-flow surface.

No source edit is requested.
