# S016386 implementation

- Task: `S016386`
- Pipeline/attempt: `P01` / `1`
- Linux source: `vendor/linux/include/uapi/linux/socket.h`
- Destination: `src/include/uapi/linux/socket.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (the frozen x86_64/AArch64 union)

The complete 38-line pinned UAPI header was read. It contains no includes or
configuration branches beyond its include guard. The translation preserves the
GPL syscall-note SPDX expression, `_K_SS_MAXSIZE`, the unsigned-short socket
family typedef, the 128-byte C layout and pointer alignment of
`__kernel_sockaddr_storage`, and all seven socket buffer/tx-rehash constants.

The C anonymous union and nested anonymous struct are represented by explicit
`#[repr(C)]` helper types because Rust has no anonymous aggregate fields. The
union retains the C alignment arm (`*mut c_void`); the data arm retains the
family field followed by 126 unsigned bytes. The lock mask remains an
expression over the two component constants, matching the source macro.

No tests, compiler, formatter, linker, runtime, or historical Lupos source was
used.
