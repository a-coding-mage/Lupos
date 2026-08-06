# S016228 application resolution

Applied by independently reopening the pinned source only. The oracle is
`vendor/linux/include/uapi/linux/lockd_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`; concrete selected use was checked
in `vendor/linux/fs/lockd/netlink.c:37-45` and the destination aggregate in
`vendor/linux/include/net/genetlink.h:78-82`. No compiler, formatter, linker,
test, runtime tool, rust-analyzer diagnostic, or historical Lupos source was
used.

## Parity finding 1 (P1): `LOCKD_FAMILY_NAME`

**Disposition: rejected in part; resolved by clarifying source semantics.**

The report correctly notes that a Rust static does not itself provide C's
implicit array-to-pointer conversion. Its stated concrete evidence is
incorrect: `struct genl_family.name` is `char name[GENL_NAMSIZ]`, not a pointer
(`include/net/genetlink.h:80`), and the sole selected C use at
`fs/lockd/netlink.c:38` is therefore C's special string-literal aggregate
initializer. That expression performs no pointer decay and does not observe
the literal's pointer identity.

The candidate retains the exact six `char` bytes, including the NUL, as stable
`[u8; 6]` storage; both frozen compile commands use `-funsigned-char`. Stable
storage is faithful for this selected header use because the only consumer
initializes an inline aggregate and cannot observe an alternate literal object
identity. The source comment now says explicitly that a distinct future
pointer-consuming translation must make its conversion at that source
expression; it does not claim Rust performs C decay. No substitute pointer
constant or invented storage-free API is warranted.

## Parity finding 2 (P2) and Rust finding R1: generated-header notices

**Disposition: accepted and fixed.**

The destination now retains all four upstream notices immediately after the
immutable rewrite provenance: the direct-edit warning, YAML generator source,
`YNL-GEN uapi header` designation, and `tools/net/ynl/ynl-regen.sh` command.
The exact dual SPDX expression and immutable task provenance remain unchanged.

## Enum / ABI / lifetime closure

Both anonymous enum declarations define enumerator constants only; they declare
no object, tag, field, pointer, allocation, ownership, or lifetime-bearing
entity. Each enumerator is a C `int` integer constant in the frozen GNU11
contexts, with values `1,2,3,4,3` for attributes and `1,2,3,2` for commands.
The `c_int` mappings and `*_MAX = __MAX - 1` expressions therefore preserve
the selected values on x86_64 and aarch64. The Phase-0 `PENDING_REVIEW`
symbol/ABI/lifetime entries for the two anonymous enums and header macros are
resolved by this source evidence as `NOT_APPLICABLE` for layout, ownership, and
lifetime; no global manifest was hand-edited during the leased-file application.

All review findings are resolved. The destination remains a complete
translation of the selected header and contains no tests, placeholders, or
unsafe code.
