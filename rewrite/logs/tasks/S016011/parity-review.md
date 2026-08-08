# Parity review — S016011, slot 1

Status: APPROVE

Reviewed only the pinned `vendor/linux/include/uapi/asm-generic/mman-common.h`,
the current `src/include/uapi/asm-generic/mman-common.rs`, the current
candidate summary, and the frozen S016011 scope/symbol/file-map records. No
compiler, formatter, test, runtime tool, or historical source was used.

## Source comparison

- Provenance matches the pinned source: SPDX is
  `GPL-2.0 WITH Linux-syscall-note`; source path, revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, architecture set `common`, and
  task ID `S016011` match the frozen task and `vendor/linux.SHA`.
- The source's author attribution is retained. No non-allowlisted Lupos or
  other branding was introduced.
- The source guard `__ASM_GENERIC_MMAN_COMMON_H` is represented by the
  path-scoped Rust module, with no exported replacement symbol. This preserves
  the header's duplicate-inclusion prevention without exposing a new UAPI
  identifier. The only local inclusion context, pinned
  `include/uapi/asm-generic/mman.h`, includes this common header before adding
  its own macros.
- Every 53 selected value macro is present once as a public `i32` constant;
  all source literals fit the C `int` range under the frozen x86_64 and AArch64
  contexts. The values and names match exactly:
  - `PROT_READ`, `PROT_WRITE`, `PROT_EXEC`, `PROT_SEM`, `PROT_NONE`,
    `PROT_GROWSDOWN`, `PROT_GROWSUP`.
  - `MAP_TYPE`, `MAP_FIXED`, `MAP_ANONYMOUS`, `MAP_POPULATE`, `MAP_NONBLOCK`,
    `MAP_STACK`, `MAP_HUGETLB`, `MAP_SYNC`, `MAP_FIXED_NOREPLACE`,
    `MAP_UNINITIALIZED`, `MAP_FILE`.
  - `MLOCK_ONFAULT`; `MS_ASYNC`, `MS_INVALIDATE`, `MS_SYNC`.
  - `MADV_NORMAL`, `MADV_RANDOM`, `MADV_SEQUENTIAL`, `MADV_WILLNEED`,
    `MADV_DONTNEED`, `MADV_FREE`, `MADV_REMOVE`, `MADV_DONTFORK`,
    `MADV_DOFORK`, `MADV_HWPOISON`, `MADV_SOFT_OFFLINE`, `MADV_MERGEABLE`,
    `MADV_UNMERGEABLE`, `MADV_HUGEPAGE`, `MADV_NOHUGEPAGE`, `MADV_DONTDUMP`,
    `MADV_DODUMP`, `MADV_WIPEONFORK`, `MADV_KEEPONFORK`, `MADV_COLD`,
    `MADV_PAGEOUT`, `MADV_POPULATE_READ`, `MADV_POPULATE_WRITE`,
    `MADV_DONTNEED_LOCKED`, `MADV_COLLAPSE`, `MADV_GUARD_INSTALL`,
    `MADV_GUARD_REMOVE`.
  - `PKEY_UNRESTRICTED`, `PKEY_DISABLE_ACCESS`, `PKEY_DISABLE_WRITE`, and
    `PKEY_ACCESS_MASK`.
- Linux symbol `PKEY_ACCESS_MASK` remains computed from
  `PKEY_DISABLE_ACCESS | PKEY_DISABLE_WRITE`, rather than being replaced with
  a literal; its resulting signed 32-bit value remains `3` and its dependency
  relationship is preserved.
- The header has no functions, objects, ABI layouts, linkage, allocation,
  locking, error, cleanup, RCU/refcount, or conditional configuration branches
  beyond the include guard. The candidate adds none of those mechanisms.

No parity findings.
