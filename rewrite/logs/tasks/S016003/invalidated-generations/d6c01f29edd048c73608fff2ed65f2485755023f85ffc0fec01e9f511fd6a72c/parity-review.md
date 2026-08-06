# Parity review — S016003

Reviewed task `S016003`, `include/uapi/asm-generic/errno.h` against the pinned
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` and candidate
`src/include/uapi/asm-generic/errno.rs`.

Result: PASS — no parity findings.

## Evidence checked

- Provenance is exact: SPDX `GPL-2.0 WITH Linux-syscall-note`, Linux source
  path, pinned revision, common architecture membership, and task ID agree
  with the queue and source mapping.
- The direct header dependency is represented by
  `pub use super::errno_base::*`, preserving the consumer-visible base errno
  definitions supplied by `#include <asm-generic/errno-base.h>`; its queue
  dependency `S016002` is DONE at the same pinned revision.
- All 102 direct errno definitions in the upstream header have a candidate
  `pub const`, with no missing or extra names.  A name/expression comparison
  found no differences.  Values cover `35..134` with the source's intentional
  unused value 58 preserved as absent.
- The three source aliases retain their exact targets and values:
  `EWOULDBLOCK = EAGAIN`, `EDEADLOCK = EDEADLK`, and
  `EFSBADCRC = EBADMSG`.  All remaining definitions retain their literal
  numeric values.
- Upstream uses unsuffixed decimal integer literals; `core::ffi::c_int` gives
  the required C `int` representation for both frozen x86_64 and AArch64
  targets.  There are no functions, layouts, linkage declarations, state,
  control-flow paths, or configuration branches beyond the include guard.
- `SYMBOLS.tsv` contains the full two-architecture inventory (206 operative
  macro rows plus four include-guard conditional rows).  `ABI.tsv` and
  `LIFETIMES.tsv` contain no records for this constant-only header, which is
  consistent with the pinned source.

No source edits were made by this reviewer.  This was a source-only review;
no build, formatting, test, or runtime command was run.
