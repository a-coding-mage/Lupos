# S015110 implementation

- Pipeline: P01; lease owner: codex-root-p01; attempt: 1
- Linux source: `include/linux/sunrpc/xprtrdma.h`
- Destination: `src/include/linux/sunrpc/xprtrdma.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `x86_64,aarch64`
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- Phase-0 identity binding: `03f3c4afb3c7edc167ddeadac5493cbee736042cb7781182d4fdf43b2b79166d`
- Source SHA-256: `fdc4755f74c381bd4d7b7711660435e61a1a83fb1e229fbd00455a754e55e3f8`
- Destination SHA-256 at sealing: `4e919cceb801fc6832fe0e0d21daaf168bed6e296fc0585fceaed8ff73a0af84`

## Translation record

The complete pinned header contains one include guard, seven operative numeric
macros, and the `rpcrdma_memreg` enum. The include guard has no Rust emission.
Unsigned C slot-table macros are represented as `u32`; the unsuffixed inline
macros retain their C `int` value type as `i32`. The enum uses `#[repr(C)]` and
the exact Linux discriminants 0 through 7. No conditional configuration branch
changes the declarations.

Required context was read from the pinned RDMA transport header, both NFS
include callers, the SUNRPC RDMA Kconfig and Makefiles, and the selected
architecture configurations. No historical Rust source, compiler, formatter,
linker, execution, test, or benchmark was used.
