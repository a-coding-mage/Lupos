# S014098 implementation record

Source: `include/linux/ioam6_genl.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete pinned header consists only of its C include guard and an include
of `uapi/linux/ioam6_genl.h`; it declares no functions, objects, types,
configuration branches, locking, ownership, or lifetime behavior.  Both frozen
architecture header closures select it through `net/ipv6/af_inet6.o`.

Translation: the Rust module re-exports
`crate::include::uapi::linux::ioam6_genl::*`.  Dependency S016196 is `DONE`;
its signed-C-`int` enum ABI and no-storage/no-lifetime records are complete for
x86_64 and aarch64.  This preserves the UAPI declarations at the kernel-header
include site and introduces no substitute netlink interface or additional
state.
