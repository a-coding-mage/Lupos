# S016270 implementation record

- Task: `include/uapi/linux/netfilter/nf_conntrack_common.h` to `src/include/uapi/linux/netfilter/nf_conntrack_common.rs`.
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Scope: `common`; both frozen kernel compile contexts define `__KERNEL__`.

The translation preserves the four C enum tags as `i32` aliases, which retains
the C enum ABI and permits all C integer values.  It exposes every enumerator
as its original global identifier, including the duplicate `IPCT_NATSEQADJ`
alias.  The selected kernel-only branches are represented by
`IP_CT_UNTRACKED`, `IPS_NAT_CLASH_{BIT,}`, `__IPCT_MAX`, and
`NF_CT_EXPECT_{DEAD,MASK}`.  The state-bit macro is preserved as a `const fn`
with the original signed-int remainder and shift expression.  All selected
flag and mask macro values are retained as `i32` constants.

No ownership, allocation, locking, lifetime, or unsafe operation is present
in this declarative UAPI header.
