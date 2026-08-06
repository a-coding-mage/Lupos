# S015110 implementation

- Lease and branch verified for pipeline `P01`; source is pinned `vendor/linux/include/linux/sunrpc/xprtrdma.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The header is selected for both frozen architectures through the built-in `fs/nfs/client.o` include closure and contains no Kconfig conditional branch beyond its C include guard.
- Translated all six macros with their C literal types: the three `U` slot-table values are `u32`; the three unsuffixed inline thresholds are `i32`.
- Translated `enum rpcrdma_memreg` as a `#[repr(C)]` fieldless Rust enum, preserving its eight ordered discriminants `0` through `7` and C enum ABI. It remains part of the kernel/user-space API; it owns no storage and has no cleanup, locking, refcount, or RCU behavior.
- Checked pinned consumers: `transport.c` initializes unsigned tunables and bounds from these constants, while `xprt_rdma.h` uses the inline maximum in a compile-time page-SGE calculation. No callable or allocation behavior resides in this header.
- No build, formatting, test, or runtime command was run.
