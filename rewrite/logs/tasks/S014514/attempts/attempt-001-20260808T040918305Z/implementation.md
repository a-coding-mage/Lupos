# S014514 implementation

Translated `include/linux/nfs_iostat.h` into `src/include/linux/nfs_iostat.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The translation preserves the header guard as a module marker, `NFS_IOSTAT_VERS`, and both C enum names, ordering, and zero-based implicit discriminants. Both enums use `#[repr(C)]` so their representation follows the C enum ABI for the approved architectures. The header contains declarations only: no allocation, ownership transfer, locking, RCU, refcounting, or cleanup behavior occurs here.

Direct pinned consumer evidence: `fs/nfs/super.c:662` formats `NFS_IOSTAT_VERS`; `fs/nfs/iostat.h:24-43` accepts both enum types as statistics-index parameters. Source citations: `vendor/linux/include/linux/nfs_iostat.h:22-122`, `vendor/linux/fs/nfs/iostat.h:24-43`, and `vendor/linux/fs/nfs/super.c:662`.
