# S016267 implementation

Translated `include/uapi/linux/netdev.h` at pinned revision `425f94c2954b1fe80ebdbf9b29854e89750355df` to its leased path. The common x86_64/AArch64 UAPI header contains six tagged C enums, eight anonymous numeric enum namespaces, and four object-like string/integer macros.

Tagged enums are represented by distinct `#[repr(transparent)]` `c_int` newtypes so their C enum ABI and tag identity are retained; anonymous enum members remain `c_int` constants. String-literal macros preserve their terminating NUL byte. The zero-valued anonymous `__NETDEV_A_XSK_INFO_MAX` and its consequent `-1` public maximum are preserved.

Read-only inventory comparison found all 143 upstream `NETDEV_*`/`__NETDEV_*` enumerators and macros represented exactly once in the candidate. No compilation, formatting, test, or runtime command was run.
