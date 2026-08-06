# Rust review — S016567

Reviewed `src/include/xen/interface/features.rs` against the complete pinned
`include/xen/interface/features.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`. This review is source-read-only;
no build, test, formatter, or source edit was run.

## Findings

1. **High — all active macro constants have the wrong Rust scalar type.** The
   source definitions at `features.h:17`, `:23`, `:31`, `:34`, `:40`, `:43`,
   `:46`, `:52`, `:55`, `:58`, `:61`, `:64`, `:72`, `:75`, `:84`, `:97`,
   `:98`, and `:100` are unsuffixed decimal integer literals. Each expands as
   a C `int`, including `XENFEAT_NR_SUBMAPS`, which is used in signed `int`
   loop arithmetic (for example `drivers/xen/sys-hypervisor.c:341`) and whose
   other feature constants are passed to `xen_feature(int flag)`
   (`include/xen/features.h:19-22`). The candidate instead declares every
   constant as `u32`. That changes signedness and prevents a direct
   representation of the source expression at consumers expecting `i32`;
   implicit Rust coercion cannot reproduce C's contextual integer conversions.
   Represent these literals as `i32` (or leave their types unannotated so the
   Rust integer fallback is `i32`) and require explicit, local conversions only
   where a translated C context requires another type.

2. **Medium — required upstream copyright notice is absent.** The source
   header records `Copyright (c) 2006, Keir Fraser <keir@xensource.com>` at
   `features.h:7`. The candidate retains the MIT SPDX identifier but drops
   this relevant upstream notice, contrary to the translation rule requiring
   retention of relevant upstream copyright notices. Restore the notice in the
   translated-file prologue without changing the immutable provenance lines.

## Checked and not findings

- The source's apparent `XENFEAT_grant_map_identity` definition is inside a
  block comment (`features.h:66-69`); correctly no Rust constant exists for it.
- All active macro identifiers and literal values 0 through 11 and 13 through
  17, plus `XENFEAT_NR_SUBMAPS = 1`, are present.
- The candidate provenance path, task ID, architecture, Linux SHA, SPDX MIT
  identifier, and lack of unsafe/FFI/layout/test constructs are correct.
