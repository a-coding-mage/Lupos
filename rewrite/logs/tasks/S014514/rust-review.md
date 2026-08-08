# Rust source review — S014514 (attempt 2, slot 2)

Reviewer: `rust_p01_s014514`  
Role: Rust reviewer  
Disposition: FINDINGS

## Sources inspected

- `vendor/linux/include/linux/nfs_iostat.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`
- `src/include/linux/nfs_iostat.rs`
- `rewrite/logs/tasks/S014514/candidate.diff`
- Frozen S014514 rows in `rewrite/SCOPE.tsv`, `rewrite/FILE_MAP.tsv`,
  `rewrite/SYMBOLS.tsv`, `rewrite/LIFETIMES.tsv`, and `rewrite/ABI.tsv`

## Finding RR-001 — `NFS_IOSTAT_VERS` is not preserved as a C-string-compatible value

Linux symbol: `NFS_IOSTAT_VERS` (`include/linux/nfs_iostat.h:25`).

The C macro expands to the string literal `"1.1"`: an array expression with a
trailing NUL byte that can decay to `const char *` at C call sites.  The Rust
candidate instead exposes `pub const NFS_IOSTAT_VERS: &str = "1.1";`.  A Rust
`&str` is a fat reference carrying a byte pointer and a length, has no C ABI,
and its byte sequence does not include the C string literal's required trailing
NUL.  It therefore cannot preserve a C-string/FFI use of this operative macro
without a separate conversion and changes both representation and call-site
semantics.

The frozen symbol inventory identifies `NFS_IOSTAT_VERS` as an operative macro
for both x86_64 and aarch64; no ABI record establishes an exception for this
macro.  Replace this representation with one whose exposed form and any FFI
use preserve the literal's NUL-terminated C-string semantics, then have the
applier establish the precise Rust-side access contract from the selected
callers.

## Manual Rust-semantics audit

All enumerator names and source-order values in both C enums are present as
`i32` constants, including `__NFSIOS_BYTESMAX = 8` and
`__NFSIOS_COUNTSMAX = 27`.  The header declares no objects, callbacks,
ownership transfer, synchronization, allocation, pointer arithmetic, or
unsafe operation; the candidate introduces none.  No `unsafe` block, `Drop`,
interior mutability, `Send`/`Sync` claim, panic path, or allocation path is
present to approve.  The type aliases erase the distinction between the two C
enum tags; because the frozen ABI rows still mark their representation and
header context `PENDING_REVIEW`, the applier must close that record with
selected caller/ABI evidence before any final acceptance.  This review does
not treat that open manifest record as established ABI evidence.

No compiler, formatter, test, runtime, linker, rust-analyzer, or historical
source was used.
