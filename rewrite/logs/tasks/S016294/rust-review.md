# Rust review — S016294 (slot 2)

Reviewer: `gpt-5.6-terra`, high reasoning effort. This was a source-only
review of `src/include/uapi/linux/netfilter/xt_state.rs` against pinned Linux
`425f94c2954b1fe80ebdbf9b29854e89750355df`. No compiler, formatter,
rust-analyzer, linker, test, debugger, or runtime tool was used.

## Result

No Rust ownership, representation, layout, arithmetic, macro-semantics, or
provenance finding.

## Evidence reviewed

- Provenance is exact: the candidate identifies the pinned UAPI source, frozen
  revision, x86_64 architecture, and task at
  `src/include/uapi/linux/netfilter/xt_state.rs:1-5`.
- The UAPI source contains precisely the three state macros and a one-member
  `struct xt_state_info` whose member is C `unsigned int`
  (`vendor/linux/include/uapi/linux/netfilter/xt_state.h:5-12`). The candidate
  retains all three expressions as `c_int` and represents the struct with
  `#[repr(C)]` and `c_uint` (`src/include/uapi/linux/netfilter/xt_state.rs:20-31`).
  For this frozen x86_64 target, the latter is the C `unsigned int` ABI type;
  there are no omitted members, padding-sensitive fields, bitfields, pointers,
  or ownership-bearing fields.
- The C connection-state sequence fixes `IP_CT_IS_REPLY` to 3 and
  `IP_CT_NUMBER` to 4 (`vendor/linux/include/uapi/linux/netfilter/nf_conntrack_common.h:7-35`).
  The imported Rust wrapper is `#[repr(transparent)]` over `c_int` and assigns
  the same values (`src/include/uapi/linux/netfilter/nf_conntrack_common.rs:15-39`).
  Therefore the candidate's `ctinfo.0 % IP_CT_IS_REPLY.0 + 1` and
  `IP_CT_NUMBER.0 + 1` retain the C integer expression and yield the required
  state bits (`src/include/uapi/linux/netfilter/xt_state.rs:17-26`).
- `XT_STATE_BIT` evaluates its argument once in both forms. Its explicit
  `unsafe` precondition preserves the C macro's invalid-shift undefined-behavior
  contract instead of turning invalid input into a safe checked API
  (`src/include/uapi/linux/netfilter/xt_state.rs:13-22`). The pinned consumer
  uses these results only as the positive state masks before the `unsigned int`
  `statebit` assignment and `statemask` mask operation
  (`vendor/linux/net/netfilter/xt_state.c:23-35`), consistent with the retained
  signed macro expression and unsigned layout field.

The frozen inventory records the same selected macros and `xt_state_info`
source locations (`rewrite/SYMBOLS.tsv`, S016294 rows) and the ABI row records
the same struct source declaration (`rewrite/ABI.tsv`, S016294 row). The
candidate is acceptable from this Rust-review slot; final closure of the
inventory's `PENDING_REVIEW` fields remains the applier's required workflow
step.
