# S016228 implementation record

- Task and destination: `S016228`, `src/include/uapi/linux/lockd_netlink.rs`.
- Oracle: `vendor/linux/include/uapi/linux/lockd_netlink.h` at pinned revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Scope: common x86_64/AArch64 UAPI header; no task dependencies.

The destination freshly maps both anonymous enum enumerator sequences and the
two family macros.  The string-literal macro is represented as its NUL-terminated
unsigned-byte array, preserving C array-to-pointer decay for its consumers.  The
enumerator constants use `c_int`, matching the frozen GNU11 C enum integer
representation; the `*_MAX` constants preserve their source expressions.

Source context inspected: generated lockd generic-netlink kernel header and
implementation (`fs/lockd/netlink.h`, `fs/lockd/netlink.c`), the set/get
consumers in `fs/lockd/svc.c`, the lockd Kbuild selection, and both frozen
configurations (`CONFIG_LOCKD=y`, `CONFIG_LOCKD_V4=y`).  No conditional branch
in this UAPI header varies between the selected configurations.

No compiler, formatter, linker, test, runtime command, or historical Rust
source was used.
