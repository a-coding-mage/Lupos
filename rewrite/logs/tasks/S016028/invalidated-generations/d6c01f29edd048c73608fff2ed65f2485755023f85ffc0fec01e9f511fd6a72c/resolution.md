# Resolution — S016028

## Inputs reopened by the applier

- Pinned source: `vendor/linux/include/uapi/asm-generic/termbits-common.h:1-66`
  at `425f94c2954b1fe80ebdbf9b29854e89750355df` (the value in
  `vendor/linux.SHA`).
- Frozen common-header task evidence: `rewrite/SCOPE.tsv`,
  `rewrite/FILE_MAP.tsv`, and both frozen configurations cited by the S016028
  rows.
- UAPI consumer/type context:
  `vendor/linux/include/uapi/asm-generic/termbits.h:5-37` includes this header
  and defines `tcflag_t` as `unsigned int`; selected consumers use flags in
  those `tcflag_t` words, e.g. `net/nfc/nci/uart.c:413-420` and
  `net/bluetooth/rfcomm/tty.c:864-866`.

## Parity-review disposition

The parity report correctly established that the source has two scalar aliases,
45 functional macros, no runtime state, no selected feature/architecture
branch, and no aggregate or callable ABI.  Its ACCEPT conclusion is superseded
only as to its literal-category rationale: matching the eventual `tcflag_t`
conversion at selected flag uses is not sufficient to preserve each UAPI
macro's own public C expression category.

## Rust-review disposition

RUST-001 is **accepted and fixed**.  `termbits-common.h:9-64` uses unsuffixed
integer literals.  On both frozen targets, every literal from `0x001` through
`0x40000000`, the baud selectors, `IBSHIFT`, and the flow/flush selectors is
representable as C `int`; `EXTA` and `EXTB` therefore remain `int` aliases of
their baud selectors.  `0x80000000` in `CRTSCTS` instead has C type `unsigned
int`, the first applicable type for that non-decimal literal.  The Rust source
now exposes the 44 `int`-category macros as `i32` and `CRTSCTS` as `u32`, while
preserving every source name, value, and `EXTA`/`EXTB` alias.  This keeps the
literal categories before a caller performs the same explicit conversion that
the C usual arithmetic conversions require for a `tcflag_t` operation.

`cc_t = u8` and `speed_t = u32` remain the exact fixed-width mappings for
`unsigned char` and `unsigned int`; the including UAPI declaration is the
source citation for the latter 32-bit type.  This header itself introduces no
object, ownership transfer, allocation, locking, RCU, refcount, callback,
Drop timing, FFI function, aggregate layout, packing, or alignment family.

## Progressive-record closure

All S016028 rows in `rewrite/SYMBOLS.tsv`, `rewrite/ABI.tsv`, and
`rewrite/LIFETIMES.tsv` are now `COMPLETE`, citing the pinned source and the
frozen target configurations.  The symbols record the unconditional include
guard as having no Rust public item and records every functional macro's C
literal category.  The ABI records specify `cc_t` as an 8-bit public alias and
`speed_t` as a 32-bit public alias; the lifetime records specify no runtime
object or synchronization.  No S016028 row exists in `rewrite/DRIVER_ABI.tsv`:
this UAPI header is not a driver-object contract, so that family is not
applicable.

No compiler, formatter, linker, test, emulator, debugger, runtime command, or
benchmark was run.
