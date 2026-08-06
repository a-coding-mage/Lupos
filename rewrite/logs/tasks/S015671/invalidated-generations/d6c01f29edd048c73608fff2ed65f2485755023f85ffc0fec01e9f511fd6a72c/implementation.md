# Implementation — S015671

Source: `vendor/linux/include/net/tls_prot.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected common header has no configuration-controlled body beyond its C
include guard.  Its three anonymous enum declarations were mapped to public
`i32` constants: C enumerator constants are `int` constant expressions, and
the frozen x86_64 and AArch64 targets both use 32-bit `int`.  The original
names and values are retained exactly; no layout-bearing type, storage,
linkage, runtime behavior, or unsafe operation is present.

Pinned consumers inspected: `net/handshake/alert.c`, `net/handshake/tlshd.c`,
`net/sunrpc/svcsock.c`, `net/sunrpc/xprtsock.c`, and
`include/trace/events/handshake.h`.  Frozen configurations select
`CONFIG_NET_HANDSHAKE=y` and leave `CONFIG_TLS` unset; the header remains
selected through the handshake and SUNRPC header closure on both targets.
