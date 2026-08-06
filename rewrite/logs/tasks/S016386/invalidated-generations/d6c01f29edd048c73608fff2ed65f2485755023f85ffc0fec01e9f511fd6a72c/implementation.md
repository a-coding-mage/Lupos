# Implementation — S016386

Translated `include/uapi/linux/socket.h` to `src/include/uapi/linux/socket.rs`
from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete unconditional common UAPI header was translated for both frozen
x86_64 and AArch64 configurations. `__kernel_sa_family_t` remains the C
`unsigned short` (`u16`). `__kernel_sockaddr_storage` retains the source
aggregate's 128-byte storage calculation and pointer-controlled default
alignment through explicit `#[repr(C)]` nested Rust aggregate types. The C
anonymous union and struct have implementation-only Rust names because Rust
has no anonymous aggregate members; their fields and representations are
otherwise preserved exactly.

All six object-like socket-buffer and transmit-rehash macros retain their C
`int` values, including the mask expression. The include guard has no Rust
runtime or ABI equivalent. There are no configuration branches, functions,
storage instances, ownership transitions, locking operations, or branding
changes in this header.

Relevant pinned consumers reviewed include UAPI RDMA, IPv4, MPTCP, TCP, and
multicast address structures, plus the socket-core users of the lock and
rehash constants. No compiler, formatter, test, runtime, or build command was
run.
