# Resolution — S016005

Source rechecked directly against pinned
`vendor/linux/include/uapi/asm-generic/hugetlb_encode.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`. This was a source-only
application; no compiler, formatter, linker, test, runtime command, or
diagnostic was invoked.

## Review dispositions

### P1 — SPDX identifier: disproved

The cited premise is not present in the authoritative selected file. Its line
1 is `#ifndef _ASM_GENERIC_HUGETLB_ENCODE_H_`; the complete 37-line header has
no SPDX comment. The `GPL-2.0 WITH Linux-syscall-note` comment quoted by the
parity report occurs instead at line 1 of consumer headers such as
`include/uapi/linux/mman.h`, `include/uapi/linux/memfd.h`, and
`include/uapi/linux/shm.h`; it is not a license identifier of this task's
source file. The candidate therefore retains the required immutable Rust
provenance header, whose required first line is
`// SPDX-License-Identifier: GPL-2.0-only`, and no unallowlisted license
substitution was made for this source.

### P2 — unsuffixed base macro signedness: accepted and corrected

Pinned-source lines 20 and 21 define `HUGETLB_FLAG_ENCODE_SHIFT` as `26` and
`HUGETLB_FLAG_ENCODE_MASK` as `0x3f`, without a suffix. Both fit in the
frozen-target C `int`, so their direct macro expressions have signed 32-bit
`int` type. The candidate now declares these two constants as `i32`.

The thirteen macros at source lines 23–35 retain `u32`: each has an `N U`
unsigned-int left operand and the same `HUGETLB_FLAG_ENCODE_SHIFT` value of
26. Their names and exponents remain respectively 14, 16, 19, 20, 21, 23, 24,
25, 28, 29, 30, 31, and 34; `34U << 26` consequently remains the unsigned
32-bit `0x8800_0000` flag encoding. This also preserves the signed direct
types exposed by the `MAP_HUGE_SHIFT`/`MASK`, `MFD_HUGE_SHIFT`/`MASK`, and
`SHM_HUGE_SHIFT`/`MASK` aliases in the pinned UAPI consumer headers.

### Rust-semantics report: accepted after P2 correction

No ownership, storage, ABI layout, linkage, unsafe operation, concurrency,
drop, allocation, or panic behavior is present in this constants-only header.
The selected C include guard has only repeated textual-inclusion semantics and
does not require a Rust public constant. All fifteen non-guard macro names are
represented exactly once; no Rust test configuration or placeholder is
introduced.

## Task semantic-record closure

The Phase 0 `PENDING_REVIEW` entries for this task are closed here by source
evidence without altering frozen manifests. `rewrite/SYMBOLS.tsv:320481–320516`
records the two architecture instances of the include guard and all fifteen
operative macros. The include guard is resolved as C textual-inclusion control
only; all fifteen macros are resolved as the two signed `i32` base constants
and thirteen unsigned `u32` encoding constants described above. The header
contains no functions, objects, layouts, linkage, ownership/lifetime,
locking/RCU, refcount, allocation, cleanup, or configuration-selected semantic
branch to leave pending. `rewrite/SCOPE.tsv:16006` confirms the common
translation scope and frozen aarch64/x86_64 header-closure selection.

All findings are resolved; the candidate is accepted for `DONE`.
