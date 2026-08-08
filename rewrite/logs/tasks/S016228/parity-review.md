# Parity review — S016228

## Scope and evidence

- Task/destination: `S016228` / `src/include/uapi/linux/lockd_netlink.rs`
- Pinned source: `vendor/linux/include/uapi/linux/lockd_netlink.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Frozen queue: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- Frozen Phase 0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- Candidate snapshot: `rewrite/logs/tasks/S016228/candidate.diff`; candidate
  source: `src/include/uapi/linux/lockd_netlink.rs`.

Manual source review only. No compiler, formatter, test, rust-analyzer, or
historical Lupos source was used.

## Finding P02-001 — `LOCKD_FAMILY_NAME` changes the macro's array-initializer contract

Linux symbol: `LOCKD_FAMILY_NAME`.

Pinned local evidence: the source macro at
`vendor/linux/include/uapi/linux/lockd_netlink.h:10` is the C string literal
`"lockd"`, hence an array of six `char` elements including its terminating NUL.
Its direct local caller initializes `.name = LOCKD_FAMILY_NAME` in
`vendor/linux/fs/lockd/netlink.c:37-39`; the destination member is
`char name[GENL_NAMSIZ]` in `vendor/linux/include/net/genetlink.h:78-81`, with
`GENL_NAMSIZ` equal to 16 in `vendor/linux/include/uapi/linux/genetlink.h:8`.
That C aggregate initialization copies the literal's six elements and
zero-initializes the remaining ten `char` elements of the 16-byte member.

Candidate evidence: `src/include/uapi/linux/lockd_netlink.rs:9` declares
`pub const LOCKD_FAMILY_NAME: &[u8; 6] = b"lockd\\0";`. This is a borrowed
slice/reference value, not the C string-literal array expression. It has no
fixed 16-`char` aggregate-initializer representation and cannot directly
preserve the `.name` initialization above; it also substitutes `u8` for C
`char`. The candidate snapshot's claim that its NUL-terminated byte array
fully translates the header omits this required use-site contract.

Affected semantic-closure keys:
`SC1-5376d0cac64bf862d5c3371e1bf3658e801935356c1bbdc9abd0124ed91ddebc`,
`SC1-f3bca7eb88f8c64c63a899011551332b36734e50b28537b35cd5d7f7c61654c6`,
`SC1-7cb16858b01562e44f3e06e4bd4ff3b9df5e1d32cbcf6e34106e6274ea9f3132`,
and `SC1-5e5d1b5a6993bd0f90fdb5971c839172a39ee02da955fa4f55a65ba4f598850e`.

Required resolution: expose a representation and initializer contract that
can initialize the Linux-compatible 16-`char` generic-netlink name field with
`"lockd\\0"` followed by zero padding, without changing `LOCKD_FAMILY_NAME`'s
observable macro use.

## Finding P02-002 — `_UAPI_LINUX_LOCKD_NETLINK_H` include-guard contract is unmapped

Linux symbols: `_UAPI_LINUX_LOCKD_NETLINK_H`, `ifndef@7`, and `endif@30`.

Pinned local evidence: `vendor/linux/include/uapi/linux/lockd_netlink.h:7-8`
tests and defines `_UAPI_LINUX_LOCKD_NETLINK_H`, and line 30 closes that guard.
The guard is the header's only selected conditional and is an operative macro
in the frozen `SYMBOLS.tsv` inventory for both architectures. It prevents
repeated inclusion from redeclaring both anonymous enums and their ordinary
identifier-namespace enumerators.

Candidate evidence: after provenance, `src/include/uapi/linux/lockd_netlink.rs:7-22`
contains constants only. It has neither an equivalent module/include boundary
nor a representation of `_UAPI_LINUX_LOCKD_NETLINK_H`; the candidate snapshot
also claims no selected branch was omitted. Rust has no C preprocessor, but
that does not establish that this selected public UAPI macro/conditional is
irrelevant or gives it a faithful mapping. The source record must either
preserve the contract through the supported Rust/C UAPI boundary or document a
source-proven equivalent; neither is present.

Affected semantic-closure keys:
`SC1-73b382ce88f33f9d7be95517349cbadd459daaa433ba4e301815a070a4633bd8`,
`SC1-9e1b28b3af3755bdbc73464d319a4d41615b6af3fd99a63a00d2de70ec316704`,
`SC1-15a57dc865035ddc992a081fce1b5b2dcda1adb45cad7ffd51908ad9c431f1e3`,
`SC1-082270722905aec1818256d3c2b6fe01e09e5081475df63a8705643d411f8fd6`,
`SC1-58072525c58f6f3693b5bb1587268eb1f1d9fb3b638829f85a449f55dfd47c89`,
`SC1-385ef78e37904246d10e1617839d069e311d80d6cc5b941f095b5aac90cecdb5`,
`SC1-b0440b5118b82cb0466b5eb601b0a7982da234528986d4e4892716722cedb582`,
and `SC1-ffebb9a4c2c542be7876e55d4c384b2aaa8c7103c02aab71fe93e97e817094f1`.

Required resolution: provide a source-evidenced equivalent for the selected
UAPI include-guard/macro boundary, or block the task if the required mixed
Rust/C UAPI boundary cannot be established from local pinned sources.

## Checked without additional parity finding

- `LOCKD_FAMILY_VERSION` is `1` in both sources.
- Both anonymous C enums have no tag or object layout to export. Their
  enumerators retain the C `int` values: server attributes `1, 2, 3, 4, 3`
  and commands `1, 2, 3, 2` in candidate lines 13-22, matching source lines
  14-19 and 23-27. No enum value/type mismatch was found.
- The header has no functions, storage objects, allocation, error, locking,
  RCU, refcount, ordering, or runtime performance path. No unauthorized
  branding was found; the allowlist has no relevant `lockd` delta.
- The SPDX expression is retained. The source's YNL generation notices are
  not copyright notices; their omission is recorded here as candidate
  provenance/context loss but is not a separate behavior finding.

## Verdict

`FINDINGS` — two source-parity defects require applier resolution before this
task can be accepted. The current candidate is not approved.
