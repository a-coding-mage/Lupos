# Parity review — S016215 / attempt 1 / slot 1

Reviewer: parity_reviewer (`gpt-5.6-terra`, high)

Scope reviewed exclusively from pinned `vendor/linux/include/uapi/linux/kernel-page-flags.h`, the frozen direct records for `S016215`, the selected in-tree consumer `vendor/linux/fs/proc/page.c`, and `candidate.diff`.

## Result: APPROVE

The pinned UAPI header contains the include guard and 27 object-like macros only.  The candidate preserves, under the same `KPF_*` names, every exported macro value from `KPF_LOCKED = 0` through `KPF_PGTABLE = 26`; it neither adds a value nor changes the deliberately unused `KPF_ERROR` value.  Each macro replacement list is an unsuffixed integer literal, whose selected x86_64 and AArch64 Linux C expression type is `int`; the candidate's explicit `i32` constants preserve that width and signedness.  The selected consumer uses these values as shift positions (`1 << KPF_*`) and as a `kpf_copy_bit` index, all within the represented 0–26 range.

The source's `_UAPILINUX_KERNEL_PAGE_FLAGS_H` guard has no data-bearing or configuration-dependent branch.  Its one-time provider role is represented by the Rust translation unit/module boundary; no distinct runtime, UAPI value, linkage, or layout contract is omitted.  The candidate's SPDX identifier, Linux source/revision, `common` architecture membership, and stable task provenance exactly match the frozen task row and pinned source.

No parity finding.
