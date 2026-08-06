# Implementation — S017908

Translated `net/ipv6/ip6_offload.h` for the frozen common x86_64/AArch64
configuration union at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source is a C include guard and four externally linked, no-argument C
function declarations returning C `int`.  Rust's module system supplies the
include-once property; the declarations are represented as one `unsafe extern
"C"` block with unchanged symbol spellings and `core::ffi::c_int` return type.
This preserves the C ABI (signed 32-bit `int` on both frozen targets) and makes
the calls explicitly unsafe because they enter kernel-global initialization or
teardown state owned by their defining translation units.

Pinned definitions and consumers inspected:

- `net/ipv6/exthdrs_offload.c`: `ipv6_exthdrs_offload_init` registers extension-header offloads with rollback on error.
- `net/ipv6/udp_offload.c`: `udpv6_offload_init` installs UDPv6 callbacks and `udpv6_offload_exit` removes them.
- `net/ipv6/tcpv6_offload.c`: `tcpv6_offload_init` installs TCPv6 callbacks.
- `net/ipv6/ip6_offload.c`: invokes TCPv6 and extension-header initializers during IPv6 packet-offload initialization.
- `net/ipv6/af_inet6.c`: invokes UDPv6 initialization and its matching cleanup on later initialization failure.

The selected header is consumed by `af_inet6.c`, `exthdrs_offload.c`,
`ip6_offload.c`, `tcpv6_offload.c`, and `udp_offload.c`; the frozen
configurations set `CONFIG_NET=y`, `CONFIG_INET=y`, and `CONFIG_IPV6=y` for
both architectures.  No task-specific ABI, lifetime, or driver-ABI rows exist;
the symbols inventory contains only the source guard and its operative macro,
which are accounted for by Rust module identity rather than emitted ABI.

No compiler, formatter, analyzer, build, test, or historical Rust source was
used.
