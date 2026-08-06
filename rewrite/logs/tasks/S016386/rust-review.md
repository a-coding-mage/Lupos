# Rust source review — S016386

Scope reviewed: pinned `vendor/linux/include/uapi/linux/socket.h`, current
`src/include/uapi/linux/socket.rs`, the frozen queue row for S016386, and
pinned in-tree uses needed to assess the UAPI storage type.  The queue maps
this common-architecture task to `include/uapi/linux/socket.h` and
`src/include/uapi/linux/socket.rs`.  This replacement review did not read any
prior review report, incident record, or compiler-derived material.

Verdict: **findings require resolution before acceptance.**

## Findings

1. **F01 — `_K_SS_MAXSIZE` has the wrong public expression type**

   - Linux evidence: `include/uapi/linux/socket.h:8` defines an unsuffixed C
     integer literal macro.  Its value therefore has C `int` type until a
     particular use applies the C conversion rules.  In the array bound at
     line 21, `sizeof(unsigned short)` is `size_t`, so that *use* converts the
     literal for the subtraction; this does not change the macro's type in
     other expressions.
   - Candidate evidence: `src/include/uapi/linux/socket.rs:10-14` expressly
     changes the public constant to `usize`, and line 30 consequently uses it
     as a pointer-width unsigned value.
   - Impact: code using the translated macro outside this one array has a
     different Rust integer type, sign/promotion behavior, and operator
     compatibility from the corresponding C macro.  This is a semantic API
     change, not merely an array-length adaptation.
   - Required resolution: retain an `i32` representation for the C `int`
     macro and perform an explicit, local conversion only in the Rust array
     length expression.

2. **F02 — the `char` payload is represented as `i8` without source evidence
   for that signedness**

   - Linux evidence: `include/uapi/linux/socket.h:21` declares `char __data`.
     C source does not make plain `char` synonymous with signed `char`; its
     signedness is target/compiler ABI-defined.
   - Candidate evidence: `src/include/uapi/linux/socket.rs:30` fixes the
     element type to `i8`, while lines 20-22 claim the member types are exact.
   - Impact: layout remains one byte per element, but reads, conversions, and
     APIs that expose the public data field acquire signed semantics that are
     not established by the header.  The reviewed task covers both frozen
     architectures, and no ABI record was present in the available source
     material to justify this hard-coded signedness.
   - Required resolution: model the member with `core::ffi::c_char`, or attach
     task-local frozen ABI evidence establishing that plain C `char` is `i8`
     for every supported target before retaining `i8`.

3. **F03 — alignment-only raw-pointer union member suppresses `Send` and
   `Sync` for a C byte-storage ABI type**

   - Linux evidence: `include/uapi/linux/socket.h:16-27` uses `void *__align`
     only as the alternative member that supplies the storage's required
     alignment.  The object is copied/embedded as address storage throughout
     pinned network and UAPI structures; for example, it is embedded in the
     UAPI structs listed in `include/uapi/linux/in.h:216-239` and
     `include/uapi/linux/tcp.h:390,417,434,468`.
   - Candidate evidence: `src/include/uapi/linux/socket.rs:35-39` represents
     that member as `*mut c_void`.  Rust raw pointers do not auto-implement
     `Send` or `Sync`, so the outer union and storage struct inherit neither
     auto trait even though the pointer is not an owned referent or a
     synchronization mechanism in Linux.
   - Impact: translated kernel data containing this UAPI storage cannot be
     sent or shared across CPU/thread boundaries under Linux's actual locking
     protocol.  This is an unintended Rust ownership restriction arising from
     an alignment device, rather than a Linux lifetime rule.
   - Required resolution: establish the intended cross-context contract and,
     if—as the source indicates—the storage is inert bytes plus alignment,
     add narrowly justified `unsafe impl Send` and `unsafe impl Sync` for the
     storage representation (with safety documentation that no union pointer
     member is dereferenced or owned).  Do not replace the union with a
     different container or silently assume thread affinity.

## Manual checks with no finding

- The alias `__kernel_sa_family_t = u16` matches the pinned `unsigned short`
  declaration for the frozen Linux targets.
- Subject to the ordinary frozen x86_64/AArch64 C ABI assumptions, the nested
  `#[repr(C)]` struct is 128 bytes (`u16` plus 126 bytes), and the `#[repr(C)]`
  union's pointer member provides pointer alignment while retaining a 128-byte
  extent.  This follows the source definitions; no compiler/layout tool was
  invoked.
- The named Rust aggregates necessarily make C's anonymous-member access
  explicit and union-field access unsafe in future Rust callers.  That is not
  itself a layout defect, but callers must preserve the active-member and
  initialization invariant rather than manufacture references to an inactive
  union field.
- This file contains no `unsafe` block or `unsafe fn`, no allocation, no
  bounds-indexing operation, no `Drop` implementation, and no callback,
  refcount, interrupt, or RCU mechanism to audit.  `Copy`/`Clone` introduce no
  allocation or drop timing here; callers still need `MaybeUninit` where Linux
  permits address-storage bytes to remain uninitialized.
- The remaining socket constants use `i32`, matching their unsuffixed C
  `int`-expression form; their values and mask expression match
  `socket.h:29-36`.

Review method: source inspection only.  No compilation, formatting, tests,
rust-analyzer/compiler diagnostics, or other executable validation was run,
requested, or used.
