# Parity review — S016270, slot 1

Verdict: APPROVE. No parity findings.

I reviewed the current attempt-1 candidate
`src/include/uapi/linux/netfilter/nf_conntrack_common.rs` against the pinned
source `vendor/linux/include/uapi/linux/netfilter/nf_conntrack_common.h` at
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` (source SHA-256
`9a595d51a7c7255e999be907c8e3bbd0bdb35464997ccaae1f9e34d000cecd69`).
The candidate-diff binding is
`9438e0a2880fb10b9c48bd5fa44ecfbce2266f760edb6c33780afecc0f69ee43`.
The sealed proposal has 391 records, SHA-256
`984cf8da3dca422748bcff56cea18e03b0e3215d47f20b3945776aa3a92f1e4c`,
and is bound to attempt 1 / P02, Phase 0 identity
`0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`,
and queue fingerprint
`cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.

Checks completed:

- Source lines 7–35 map `enum ip_conntrack_info` as signed `i32` constants
  with exact sequence and arithmetic: established 0, related 1, new 2,
  `IP_CT_IS_REPLY` 3, reply aliases 3 and 4, and `IP_CT_NUMBER` **5**.  The
  selected kernel arm preserves `IP_CT_UNTRACKED = 7`; the mutually exclusive
  non-kernel `IP_CT_NEW_REPLY` is correctly not emitted.
- Source lines 37–39 preserve both fixed state masks and
  `NF_CT_STATE_BIT(ctinfo)` as a single-evaluation expression macro.  Its
  Rust expansion retains the signed `int` left operand, remainder/addition
  grouping, and shift expression: `1i32 << (($ctinfo % IP_CT_IS_REPLY) + 1)`.
- Source lines 42–130 map every `ip_conntrack_status` bit index, derived flag,
  NAT mask, NAT-done mask, `IPS_UNCHANGEABLE_MASK`, and `__IPS_MAX_BIT = 16`
  exactly.  The selected `__KERNEL__` aliases `IPS_NAT_CLASH_BIT` and
  `IPS_NAT_CLASH` retain their aliases of `IPS_UNTRACKED_BIT` and
  `IPS_UNTRACKED` respectively.
- Source lines 133–155 retain all event values and aliases, including
  `IPCT_NATSEQADJ = IPCT_SEQADJ`, the selected `__IPCT_MAX = 12`, and
  expectation events 0 and 1.  Source lines 158–166 retain the four exact
  expectation flags and kernel-only three-flag mask.
- The UAPI SPDX expression and immutable provenance match the task.  The
  provider wrapper `include/linux/netfilter/nf_conntrack_common.h:5-6`
  directly includes this UAPI header.  Its source C include guard has no Rust
  item because a Rust module is included once; this is the faithful
  module-level mapping.  The frozen header-closure records select the header
  for both architectures (aarch64: 1,722 consumers; x86_64: 636 consumers).
  The source's `__KERNEL__` branches at lines 30–34, 100–107, 147–149, and
  162–166 are consistently mapped to the frozen kernel configuration; the
  candidate does not expose a conflicting userspace arm.
- Proposal-key mapping is complete: scope key
  `SC1-702eaf86a03ce5736b392367fd0d089d1265db1984a0809acd18dec40ea3a6e1`;
  163 selected-symbol records per architecture; and 16 ABI plus 16 lifetime
  records per architecture.  All 391 currently sealed keys are COMPLETE and
  bound to the candidate-diff hash above; no semantic record requires a
  finding mapping.

No compiler, formatter, linker, test, runtime, or diagnostic was invoked.
