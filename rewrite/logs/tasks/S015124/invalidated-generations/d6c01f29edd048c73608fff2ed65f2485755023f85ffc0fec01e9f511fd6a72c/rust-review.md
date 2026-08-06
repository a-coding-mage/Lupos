# Rust review — S015124

Reviewed `src/include/linux/sys.rs` against pinned
`vendor/linux/include/linux/sys.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 header-closure
record, and its two selected consumers
`arch/x86/entry/syscall_32.c` and `arch/x86/entry/syscall_64.c`.

Result: **no Rust-semantic findings.**

The source header's only non-guard content is nine obsolete `_sys_*` macro
aliases inside `#ifdef notdef`.  `notdef` is neither defined by this header nor
by the frozen selected-consumer context, so the active preprocessor surface
contains no declarations, types, linkage, layout, or runtime contract.  The
include guard itself is likewise a C-preprocessor-only mechanism.  An empty
Rust destination therefore introduces no missing active identifier or linkage
requirement.  The candidate has the exact required source/revision/task and
x86_64 provenance, retains the source SPDX identifier, and its explanatory
comment accurately limits the omission to the inactive branch.

No ownership, unsafe, FFI, layout, panic, or module-level semantic issue is
introduced by this empty active mapping.
