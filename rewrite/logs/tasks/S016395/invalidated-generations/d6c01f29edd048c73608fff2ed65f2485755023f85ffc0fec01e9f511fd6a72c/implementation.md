# S016395 implementation, attempt 2

Source: `vendor/linux/include/uapi/linux/sunrpc_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The full generated UAPI header was reread with its immediate SunRPC netlink
consumers (`net/sunrpc/netlink.c` and `net/sunrpc/netlink.h`). The destination
maps the named `enum sunrpc_cache_type`, all six anonymous enum domains, and
the three selected public macros. Every enumerator remains a `c_int` constant:
the frozen x86_64 and AArch64 ABI records establish 4-byte signed C `int` enum
representation because `-fshort-enums` is absent. The three string-literal
macros retain their exact bytes, trailing NUL, and fixed array lengths as `u8`
arrays; their consuming Rust translation performs pointer decay explicitly at
the use site.

The attempt-2 reconciliation removed the three non-upstream `*_PTR`
convenience constants. They were not C-header declarations or macros and
would have expanded the translated public surface. No lifetime-bearing object,
storage protocol, synchronization, function, or unsafe operation exists in
this declaration-only header. The task-specific ABI and lifetime records are
complete for both frozen architectures.

No build, formatter, compiler, test, or runtime command was run.
