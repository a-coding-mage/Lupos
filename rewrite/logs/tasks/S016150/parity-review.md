# Parity review — S016150 / attempt 1 / P02

Reviewer: parity_reviewer (`gpt-5.6-terra`, high)

Scope reviewed: `vendor/linux/include/uapi/linux/hsr_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the aarch64 frozen records, and
the candidate snapshot for `src/include/uapi/linux/hsr_netlink.rs`.

Result: **FINDINGS**. No compiler, formatter, test, runtime command, or
historical source was used.

## P1 — Candidate snapshot does not contain the candidate source

`candidate.diff` has only its `/dev/null` diff header and two added prose
lines. It does not include the provenance header or any `HSR_A_*`/`HSR_C_*`
definition that is present in the candidate file. Consequently the required
immutable review input does not identify the candidate being reviewed, so it
cannot evidence a source-complete translation or support a final candidate
binding.

Evidence: `rewrite/logs/tasks/S016150/candidate.diff:1-5`; candidate source
`src/include/uapi/linux/hsr_netlink.rs:1-35`.

## P2 — The anonymous-enum ABI closure is asserted without an established representation

The header has two selected anonymous enum declarations at lines 21 and 39.
The candidate changes each enumerator into a Rust `i32` constant and states
that this retains a signed C-`int` value domain. The frozen ABI records for
both declarations instead retain layout, alignment, and export kind as
`PENDING_REVIEW`. The proposal changes those fields to the literal
`SOURCE_REVIEWED_VALUE`, rather than a source-backed ABI value. The pinned
header and the selected direct consumer establish the numeric enumerator
sequence, but do not establish that this Rust type substitution is the exact
frozen C representation/consumer contract. This must be resolved with the
required pinned ABI evidence before it can be marked complete.

Evidence: `vendor/linux/include/uapi/linux/hsr_netlink.h:21-35,39-49`;
`rewrite/ABI.tsv` rows for `anonymous_enum@21` and `anonymous_enum@39`;
`vendor/linux/net/hsr/hsr_netlink.c:209-216,544-571`.

Affected closure keys:

- `SC1-0a4c9c9b2e80f00f2d2b5a25a857739f03f795ebb816ffa9f8759782b8bf1b94`, `SC1-ab2d1d6cffe1147e5b295fac62031b4251be698c5d5a1e4e06acd5d5b54f95e1`, and `SC1-27fb36b8e071aed5bca22b395196219d98e5f15f85b2d37079141109874089a8`
- `SC1-58d479125c1d22fb3a9518c4e80cac2b07b3c5c8e52a7472b0eba3989ef00cd8`, `SC1-8c69a56f345454402ae823e2943dcc558ad3ec8ed0d1701c54aa6cbad5280c80`, and `SC1-09ddd53d99733ff4e1fb4d09fc74144ca90a60ef8c253b7b9846d2d878510c70`

## P3 — The selected C inclusion-guard macro has no faithful mapping

`__UAPI_HSR_NETLINK_H` is a selected operative macro controlling the C
header's repeated-inclusion semantics. The candidate supplies no C-compatible
guard or documented equivalent, while the closure proposal changes its
selection expression to the placeholder `SOURCE_REVIEWED_VALUE` and marks it
complete. Rust module loading is not itself a mapping of a C preprocessor
macro for the original driver/object boundary. Resolve the required boundary
or retain this record pending; do not claim it complete without source-backed
evidence.

Evidence: `vendor/linux/include/uapi/linux/hsr_netlink.h:14-15,51`;
`rewrite/SYMBOLS.tsv` record for `__UAPI_HSR_NETLINK_H`;
candidate `src/include/uapi/linux/hsr_netlink.rs:1-35`.

Affected closure key: `SC1-fc590a192de2ca1b902b2560abc5338e79fc723d4c9b7e01cdeefda2a73393f6`.

## P4 — Required upstream copyright/author notice was dropped

The candidate retains the SPDX identifier but omits the upstream copyright
notice and author attribution in the pinned source. The fresh-source rules
require relevant upstream copyright notices to be retained.

Evidence: `vendor/linux/include/uapi/linux/hsr_netlink.h:2-11`; candidate
`src/include/uapi/linux/hsr_netlink.rs:1-35`.
