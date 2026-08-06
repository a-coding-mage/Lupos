# Parity review — S013730, attempt 2, slot 1

Reviewed source-only against pinned `vendor/linux/include/linux/device-id/rpmsg.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 and AArch64
configuration/compile-command evidence, and direct in-tree users
(`include/linux/rpmsg.h`, `drivers/rpmsg/rpmsg_core.c`, and
`scripts/mod/file2alias.c`). No compiler, formatter, linker, test, debugger,
or historical/candidate-diff/review artifact was used.

## Finding P1 — upstream SPDX identifier was changed

- **Candidate:** `src/include/linux/device-id/rpmsg.rs:1` declares
  `SPDX-License-Identifier: GPL-2.0-only`.
- **Pinned Linux evidence:** `include/linux/device-id/rpmsg.h:1` declares
  `SPDX-License-Identifier: GPL-2.0`.
- **Impact:** The immutable source provenance is not retained exactly, contrary
  to the path-preserving translation requirement. This is an unauthorized
  license-identifier change even though it does not alter the C data layout.
- **Required resolution:** Replace the candidate SPDX identifier with the exact
  upstream identifier, `GPL-2.0`.

## Verified parity points

- `RPMSG_NAME_SIZE` is C integer literal `32`; the candidate retains an
  `c_int` value of `32`, which is a 32-bit C `int` on both frozen targets.
- `RPMSG_DEVICE_MODALIAS_FMT` retains the literal bytes `rpmsg:%s` and its C
  string terminator. Its in-tree consumers use it as a format string and in C
  literal concatenation (`rpmsg_core.c:371,424`; `file2alias.c:852`); no byte
  content was changed.
- The selected AArch64 compile commands for direct RPMSG users contain both
  `-D__KERNEL__` and `-funsigned-char`. Therefore the header's conditional
  `kernel_ulong_t` is active and its `char name[32]` representation is an
  unsigned 32-byte inline array in the frozen kernel context. `c_ulong` maps
  the C `unsigned long` field on both approved 64-bit architectures.
- `#[repr(C)]` with `[u8; 32]` followed by `c_ulong` preserves field order,
  the 32-byte name offset, native unsigned-long alignment, and the resulting
  40-byte structure layout on both approved targets. `Clone, Copy` preserves
  the C structure's ordinary by-value copy capability; it introduces no
  alternate storage or behavior.
- The candidate provenance source path, revision, architecture set, and task
  ID match the frozen mapping and `vendor/linux.SHA`.

Result: **REJECT pending resolution of P1.**
