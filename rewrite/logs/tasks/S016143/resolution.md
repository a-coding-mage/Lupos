# Resolution — S016143

Applier reopened the complete pinned
`vendor/linux/include/uapi/linux/hash_info.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, both frozen target command
records, the selected `include/crypto/hash_info.h` and
`lib/crypto/hash_info.c` consumers, and the selected UBIFS field and
initialization path. This was source-only work; no compiler, formatter, test,
or runtime command was run.

## Parity P1 / Rust R1 — tag, enumerator surface, and non-enumerator values

**Accepted and fixed.** Upstream lines 17–42 declare both the distinct C tag
`enum hash_algo` and 24 unqualified enumerator identifiers. C enumerators are
ordinary signed-`int` constant expressions, so the final source exposes every
`HASH_ALGO_*` identifier, including `HASH_ALGO__LAST`, as a public `c_int`
constant with the exact value 0 through 23. This preserves use as the array
bound in `include/crypto/hash_info.h:38-39` and as the designated indices in
`lib/crypto/hash_info.c:11-63`; no Rust variant namespace or closed variant
set remains.

The final source separately retains the C tag as public
`#[repr(transparent)] struct hash_algo(pub c_int)`. It is a four-byte,
four-byte-aligned signed-C-`int` value on both frozen targets and accepts every
such bit pattern, rather than imposing Rust fieldless-enum validity. That is
required by `fs/ubifs/ubifs.h:1493` and `fs/ubifs/auth.c:267-269`: the
`match_string()` result is stored in `auth_hash_algo` before its negative value
is checked. The same tag is used by pointer output at
`crypto/asymmetric_keys/pkcs7_verify.c:139-166`.

## Frozen ABI decision

Both retained Phase-0 command databases bind the source to LLVM 19.1.7:
`rewrite/metadata/aarch64/compile_commands.json` uses
`--target=aarch64-linux-gnu`, and
`rewrite/metadata/x86_64/compile_commands.json` uses
`--target=x86_64-linux-gnu`. Neither frozen command set contains
`-fshort-enums`. Every upstream enumerator is in signed C `int` range
(0 through 23). Together with the pinned target command evidence, this fixes
the selected ABI as a signed C `int`, size four and alignment four, for each
target. `c_int` is therefore the exact Rust backing representation; it is not
an inference from the removed `#[repr(C)]` enum.

## Record closure

All eight S016143 symbol rows now record the unconditional include guard and
the named tag/enumerator mapping as `COMPLETE`. Both ABI rows now record the
frozen size, alignment, backing representation, and selected consumer
evidence. Both lifetime rows record that this declarative header creates no
storage, ownership, cleanup, locking, RCU, or refcount contract.

`rewrite/DRIVER_ABI.tsv` and `rewrite/BLOCKERS.tsv` have no S016143 rows: this
is a UAPI header, not a driver object, and the pinned source plus frozen target
evidence resolves every task-local question. All review findings are resolved;
no finding remains open.
