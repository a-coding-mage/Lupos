Task S016168 translates the complete pinned `include/uapi/linux/if_infiniband.h`
header at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` for the
common x86_64/AArch64 configuration union.

The source contains one include guard and one operative macro. The Rust module
boundary supplies the guard behavior; `INFINIBAND_ALEN` is represented as a
public `i32` constant with value 20, preserving the C integer-literal value
domain and expression behavior. Direct pinned consumers use it as the
20-octet IPoIB hardware-address length (IPv6 address configuration and TIPC
InfiniBand media support).

Evidence inspected: the complete pinned header; `net/ipv6/addrconf.c` including
`addrconf_ifid_infiniband`; `net/tipc/ib_media.c`; `include/linux/netdevice.h`
for `addr_len`; and the frozen S016168 rows in SCOPE.tsv, SYMBOLS.tsv,
FILE_MAP.tsv, and TRANSLATION_TASKS.tsv.
