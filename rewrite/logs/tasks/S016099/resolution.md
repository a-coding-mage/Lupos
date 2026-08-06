# S016099 resolution

## Sources reopened

- `vendor/linux/include/uapi/linux/dev_energymodel.h` at pinned revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The frozen AArch64 compile command recorded for the header's consumer,
  `kernel/power/em_netlink.o`, in `rewrite/FILE_MAP.tsv`.
- `vendor/linux/kernel/power/em_netlink_autogen.c`, which consumes the family
  name and version and uses the header's integer constants as generic-netlink
  command and attribute values.

## Review dispositions

There were no source findings from either independent review, so no candidate
source edit is required.

The parity review's non-blocking evidence note is correct: the candidate and
upstream contain 29 enumerator constants in total (4 named-enum enumerators
and 25 anonymous-enum constants), rather than the 33 stated in the two
implementation summaries.  This is only a prose count error; every upstream
constant is present with its original value or `__*_MAX - 1` relation.

## Independent ABI adjudication

Both named C enum tags are represented as aliases of `core::ffi::c_int`.
Their enumerators are within the C `int` range, and the frozen AArch64 command
uses `--target=aarch64-linux-gnu` without `-fshort-enums`; its `-funsigned-char`
option does not alter `int`.  The aliases therefore retain the selected enum
integer representation while allowing the bit-flag combinations permitted by
the C interface.

`DEV_ENERGYMODEL_FAMILY_NAME` and `DEV_ENERGYMODEL_MCGRP_EVENT` remain static
`c_char` arrays containing, respectively, the exact bytes
`dev-energymodel\\0` (16 bytes) and `event\\0` (6 bytes).  The only source
characters are ASCII and NUL, so the frozen unsigned-`char` setting leaves the
stored byte sequence unchanged.  Array storage also retains C string-literal
semantics; a Rust consumer performs C's ordinary expression-context
array-to-pointer decay explicitly with `.as_ptr()`.

This header declares no data layout, ownership transfer, locking, lifetime,
RCU, refcount, or callable ABI.  The task-local ABI and lifetime decisions are
therefore: `c_int` enum/integer constants; static NUL-terminated character
arrays; all ownership and synchronization fields not applicable.

## Result

Accepted without source changes.  This is a source-pipeline completion only;
no compilation, formatting, linking, testing, or runtime command was run.
