# Parity review — S016395 (slot 1)

Result: **APPROVE**

Reviewed the complete pinned `include/uapi/linux/sunrpc_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/sunrpc_netlink.rs` and the current candidate diff.

Evidence reviewed:

- `rewrite/SCOPE.tsv` classifies this header as `RUST_TRANSLATE` for `common`;
  both frozen configurations select it through `net/sunrpc/cache.o`.
- `rewrite/SYMBOLS.tsv` inventories the include guard, four operative macros,
  named `enum sunrpc_cache_type`, six anonymous enums, and all enumerators for
  both x86_64 and aarch64. The candidate contains each selected item.
- Pinned `net/sunrpc/netlink.c`, `net/sunrpc/netlink.h`, `cache.c`, and
  `svcauth_unix.c` use the retained cache-type, attribute, command, family, and
  multicast values as the generated netlink protocol specifies. Pinned
  `Documentation/netlink/specs/sunrpc_cache.yaml` confirms the same protocol
  names, values, flags, and multicast groups.

The candidate preserves the exact family strings (`sunrpc`, `none`, `exportd`),
family version 1, `sunrpc_cache_type` discriminants 1 and 2 with `repr(i32)`,
and every anonymous-enum progression and derived `*_MAX` value: cache notify
1; IP-map 1..6/max 6; IP-map requests 1/max 1; UNIX-GID 1..5/max 5;
UNIX-GID requests 1/max 1; cache flush 1/max 1; and commands 1..6/max 6.
It contains no conditionals beyond the C header's include guard counterpart,
no runtime mechanism, ABI-bearing structure, linkage, allocation, lock,
ordering, error, lifetime, or refcount path to diverge. The immutable source,
revision, architecture, task provenance, SPDX identifier, and Linux names are
retained; `BRANDING_ALLOWLIST.tsv` contains no applicable delta.

No parity findings. No compiler, formatter, linker, test, runtime, or
rust-analyzer diagnostics were invoked or used.
