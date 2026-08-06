# Parity review — S016242

Reviewed independently against the pinned Linux source and Phase 0 records; no
compiler, formatter, linker, test, runtime command, or diagnostic was used.

## Scope and evidence

- Task `S016242` maps `include/uapi/linux/memfd.h` to
  `src/include/uapi/linux/memfd.rs`, is `RUST_TRANSLATE`, architecture `common`,
  and has the sole dependency `S016005` (`rewrite/SCOPE.tsv`, S016242;
  `rewrite/TRANSLATION_TASKS.tsv`, S016242;
  `rewrite/metadata/task_dependencies.tsv`, S016242).
- The header is selected through `mm/memfd.o` as built-in `vmlinux.a` for both
  x86_64 and aarch64 (`rewrite/metadata/header_include_edges.tsv`, entries for
  `include/uapi/linux/memfd.h`; `rewrite/metadata/header_closure.tsv`, entries
  for that header).  There is no configuration-controlled content branch in
  the source: its only conditionals are the ordinary include guard
  (`vendor/linux/include/uapi/linux/memfd.h:2-3,39`), represented by the Rust
  module boundary.

## Finding

1. **HIGH — UAPI SPDX identifier changed.**  The pinned source begins with
   `SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note`
   (`vendor/linux/include/uapi/linux/memfd.h:1`), while the candidate declares
   `GPL-2.0-only` (`src/include/uapi/linux/memfd.rs:1`).  This is not an
   allowlisted branding change (`rewrite/BRANDING_ALLOWLIST.tsv` contains only
   its header and no mappings) and fails the requirement to retain the upstream
   SPDX identifier.  Restore the exact UAPI SPDX expression.

## Verified parity items

- The candidate has all nineteen public UAPI aliases from the source: five
  unsigned memfd flags, `MFD_HUGE_SHIFT`, `MFD_HUGE_MASK`, and the twelve
  `MFD_HUGE_*` aliases (`vendor/linux/include/uapi/linux/memfd.h:8-37`;
  `src/include/uapi/linux/memfd.rs:18-44`).  It neither omits nor adds a
  `MFD_HUGE_16KB` alias; that name exists only in the included source header
  and is deliberately not re-exported by `memfd.h`
  (`vendor/linux/include/uapi/asm-generic/hugetlb_encode.h:18-31`).
- `MFD_CLOEXEC`, `MFD_ALLOW_SEALING`, `MFD_HUGETLB`, `MFD_NOEXEC_SEAL`, and
  `MFD_EXEC` preserve their `unsigned int` literal values as `u32`
  (`vendor/linux/include/uapi/linux/memfd.h:8-14`; candidate lines 18-26).
  This is the correct width for both frozen Linux targets.
- `MFD_HUGE_SHIFT` and `MFD_HUGE_MASK` remain signed integer constants (`i32`)
  matching the unsuffixed `26` and `0x3f` source macros, while every huge-page
  encoding remains `u32`, preserving the source dependency's `U`-suffixed
  left-shift semantics (`vendor/linux/include/uapi/asm-generic/hugetlb_encode.h:16-31`;
  `src/include/uapi/asm-generic/hugetlb_encode.rs:8-24`;
  candidate lines 29-44).
- Every `MFD_HUGE_*` name aliases its same-named S016005
  `HUGETLB_FLAG_ENCODE_*` public constant.  The candidate's
  `crate::include::uapi::asm_generic::hugetlb_encode` import is the Rust-path
  representation of frozen dependency S016005, whose mapped destination is
  `src/include/uapi/asm-generic/hugetlb_encode.rs`
  (`rewrite/TRANSLATION_TASKS.tsv`, S016005; candidate lines 9-14,33-44).
- Source provenance fields name the correct Linux source, frozen revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, common architecture scope, and
  task identifier (`src/include/uapi/linux/memfd.rs:2-5`).

## Conclusion

One HIGH source-parity finding requires applier resolution: restore the
upstream `GPL-2.0 WITH Linux-syscall-note` SPDX identifier.  Apart from that
identifier, the selected aliases, values, signedness, dependency aliases,
public names, and architecture-independent guard behavior match the pinned
header.
