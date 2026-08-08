# S014514 parity review — slot 1 / attempt 2

Result: FINDINGS

Reviewed only the pinned `vendor/linux/include/linux/nfs_iostat.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate
`src/include/linux/nfs_iostat.rs`, candidate diff, and the S014514 rows in the
frozen scope, symbols, lifetime, and ABI manifests.  No compiler, formatter,
linker, test, runtime, or diagnostic output was used.

## Findings

1. **F1 — `NFS_IOSTAT_VERS`: macro expression and representation changed.**
   Linux `NFS_IOSTAT_VERS` is the preprocessor macro `"1.1"` at
   `include/linux/nfs_iostat.h:25`; after expansion it is a C string literal,
   including its trailing NUL when materialized and with C string-literal
   expression behavior.  The candidate instead declares
   `pub const NFS_IOSTAT_VERS: &str = "1.1";` at
   `src/include/linux/nfs_iostat.rs:7`.  A Rust `&str` is a length-carrying
   fat reference to three UTF-8 bytes, not a C string-literal macro or a
   NUL-terminated character array/pointer expression.  This changes both the
   selected operative macro's use semantics and its ABI-relevant representation
   if a consumer uses it at a C-facing boundary.  The frozen symbols rows for
   both architectures select `NFS_IOSTAT_VERS` at Linux line 25.

2. **F2 — `_LINUX_NFS_IOSTAT`: selected include-guard macro and branch are
   omitted.**  Linux defines `_LINUX_NFS_IOSTAT` at line 23 under the
   `#ifndef _LINUX_NFS_IOSTAT` branch beginning at line 22 and closes that
   branch at line 122.  The candidate provides no mapping for this operative
   macro or its conditional state.  Rust module loading alone is not a
   declaration of `_LINUX_NFS_IOSTAT` for code that relies on the selected
   macro/conditional contract.  The frozen symbols manifest explicitly selects
   `ifndef@22`, `_LINUX_NFS_IOSTAT`, and `endif@122` for both x86_64 and
   aarch64; candidate lines 1–45 contain no corresponding symbol or condition.

3. **F3 — `enum nfs_stat_bytecounters` and `enum nfs_stat_eventcounters`:
   distinct C enum types and unestablished ABI are collapsed to `i32`.**
   Linux declares two separately named enum types at lines 62 and 91, each
   with the listed integral enumerators.  The candidate maps both names to
   `pub type ... = i32` (lines 9 and 22), which erases the two nominal types
   and permits unrestricted interchange with every `i32` and with each other.
   More importantly, it asserts an `i32` representation while each frozen ABI
   row for these Linux symbols and both architectures remains `PENDING_REVIEW`
   for layout and alignment.  Source-only review cannot establish that this
   representation, alignment, or any C-facing use is exact; accepting the
   aliases would guess across an unresolved frozen ABI record.  The source
   order and numerical values of the enumerator constants are otherwise
   present, but do not resolve the type/ABI omission.

No locking, allocation, lifetime transfer, error path, branding delta, or
linkage-bearing declaration exists in this Linux header beyond the issues
above.
