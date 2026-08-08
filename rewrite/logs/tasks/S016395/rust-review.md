# Rust source review — S016395 / attempt 1

Reviewed `vendor/linux/include/uapi/linux/sunrpc_netlink.h`, the fresh
`src/include/uapi/linux/sunrpc_netlink.rs`, the candidate record, frozen
scope/symbol/ABI/lifetime records, and narrow pinned consumers.  No compiler,
formatter, test, or runtime tool was used.

## Findings

### RUST-ENUM-CONSTANT-NAMESPACE-TYPE — reject

`sunrpc_cache_type` is represented as a Rust enum whose two discriminants are
namespaced associated variants.  In the C header, `SUNRPC_CACHE_TYPE_IP_MAP`
and `SUNRPC_CACHE_TYPE_UNIX_GID` are ordinary, file-scope enumerator
identifiers with `int`-constant semantics; they are not values of a scoped
enum type.  The candidate consequently exports neither bare integer constant,
and it makes each available value a restrictive Rust enum value.  This changes
both the public name/type contract and the permissible bit patterns of the
declared C enum type.

The pinned consumers require integer-mask behavior: `net/sunrpc/cache.c:1979`
takes `u32 cache_type`, and `net/sunrpc/svcauth_unix.c:842-847` applies the
two enumerators as masks to a `u32`.  A Rust enum variant cannot preserve that
C implicit-integer contract (and cannot model arbitrary incoming C enum
representation values) without an explicit compatibility representation and
separately exported integer constants.  `#[repr(i32)]` fixes only the selected
discriminant width; it does not restore the C identifiers, integer-constant
type, or arbitrary-representation contract.

Affected semantic keys:

- `SC1-16fd2aaee6d6cf425289972c7810e5ae419851349b448e450d19f68005cc6044`, `SC1-ed0f3aebfae9706fa37bc0987d6366666d16319a0f1a7e9438cd2f12aa566bcb`, `SC1-c1708c3726dbd62c7025ad6ebdc1f3a98b5ad2877de9f7f6c868835cb622548d`, `SC1-c49d189305de224317d39a5e12133cda4596b3949d79cdf387b8109bb87022e4` (named-enum selection/status)
- `SC1-a76b6c71d95dfb5530ca73a52cf43135e382bd07b88e9f54c2ff52ad85726291`, `SC1-ae6784ec1341476f875085aa1cb1319737169ddbaf42dd8be603cc02ffe78d76`, `SC1-83f36bbcda9ebd28e50434f62bf3a61b50f79daa428b96148502d1fa14287854`, `SC1-49d594d9e0a3bc13118ed149b8fb9e47e4e7f8b1fa44d8252dc8a141e9b33c0f`, `SC1-9298c88c7767f685152e86a5f950b9506ac3a8c9afd29699810bda402d152687`, `SC1-46a481fa1fc4c3b8c1758bd9ce21727671bb248482e07c2d315b16c31ebb806e`, `SC1-c2af3cb1c5fbfa6834b9aaed2aa16ba70ff7f5ed26f6ca0578b2579cdea292bc`, `SC1-41dc1e10bdea3f145b8fb344768c131f3d8513c939dbff5f8ea4658c8309f048` (enumerator selection/status)
- `SC1-ceb8f4ab180b5cbda3aab2641431624d39c156f9bd80178bcc2ab0cf92dcdc7b`, `SC1-d8c45dad5f4b7a280bd8e8f96def3820813982066276d4d684c509f40e210771`, `SC1-ef166c5a4ca97d61a7c1114bd2d7dc931f365af086e1d0635d7265091c3e4bfa`, `SC1-dbe314a5435b0a4b64a7b8726979e4d78b3c3ff9c729bcda3b6d704fa181b474`, `SC1-ce2946396007a46cc29b4688d2c2dc9ac6556e4ba3cd82f8db370ab228e94ed6`, `SC1-822d909a61d1d1a574e41ebbccbb2655d0217fad972aea6144e84d724d108823`, `SC1-a7b94c983231601d946391ef089ae2d580bd506224629679aa28f29640d8ae1f`, `SC1-605ae14ea99486d148b7db40d8a08525ab7748092a1e6fce968de0164d992e9c` (named-enum ABI).

### RUST-C-STRING-MACRO-ABI — reject

`SUNRPC_FAMILY_NAME`, `SUNRPC_MCGRP_NONE`, and `SUNRPC_MCGRP_EXPORTD` are C
string-literal macros.  Each expands to a NUL-terminated character array that
decays to `const char *` in a C expression.  The candidate instead exports
Rust `&str` values: these have no trailing NUL and their representation is a
data-pointer/length pair, not a C character pointer.  This is an ABI and
terminator change, not a harmless presentation change.  In the pinned direct
consumer, `net/sunrpc/netlink.c:87-89` initializes the generic-netlink
family's C character-pointer `.name` from `SUNRPC_FAMILY_NAME`.

Affected semantic keys:

- `SC1-f9aee4d6f33463d91b2cb57d5cf49790889727f1a20a4ca9556657fcdd25ee95`, `SC1-a80281f2652b7fbf58389b3e829a98d4ef7f4c26b7d58dc4a01129cb3437862e`, `SC1-7e4968f643283cf5cf734f26ed6482cd6acea645cb700beb4240dc78525161ec`, `SC1-2283af99398c30b02b7af997b5e4005d1c4a672909c377cff129734c27f3df93` (`SUNRPC_FAMILY_NAME`)
- `SC1-111acc2941ccf8f4d926301d4440e325f816dadd807653a9a19520028929f778`, `SC1-bd139a30a6c54e27da5712e34b25a46dfba6f0b2307a9c94187acf52ffc4efe4`, `SC1-df2ff3b39adf324f52b985997639c9423ebfe083c44b74bc4052d4765303d6b1`, `SC1-4ba560142227f76a8ecfbd80a199997fe2d14ef704daf60efc23d8bf002834c1` (`SUNRPC_MCGRP_NONE`)
- `SC1-12c69d7e38ff9aaeb94d2a31eb03b67ebe5b69605a695e7b31b23ff66b7d4cab`, `SC1-279cf5e7513d60fe9530f616c914d45abb58f04c5966deea72232face39b745a`, `SC1-1778ee10d5bf7546bbf0f43d82334ef481baa2cce729d820ff41636955d2fb62`, `SC1-1a45c43e3af89a887d4a0fe4f74e986f05e15a2cf4f85e69c90f428e8898b99f` (`SUNRPC_MCGRP_EXPORTD`).

No `unsafe` blocks, borrowing, allocation, callbacks, or drop behavior occur
in this header candidate.  The remaining anonymous-enum constants retain the
pinned numeric sequence and their derived `MAX - 1` arithmetic as `i32`.

Review status: FINDINGS.
