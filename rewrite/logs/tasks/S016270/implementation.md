# S016270 implementation

Translated `include/uapi/linux/netfilter/nf_conntrack_common.h` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` to `src/include/uapi/linux/netfilter/nf_conntrack_common.rs`.

The four C enum tags are represented by `i32` aliases, preserving C enum storage and permitting the source's duplicate enumerator values. Every selected enumerator and flag is an `i32` constant retaining its C expression and value. `NF_CT_STATE_BIT(ctinfo)` remains an expression macro. The frozen x86_64 and AArch64 Kbuild command contexts define `__KERNEL__`; therefore the kernel-only `IP_CT_UNTRACKED`, `IPS_NAT_CLASH_*`, `__IPCT_MAX`, and `NF_CT_EXPECT_{DEAD,MASK}` arms are present, while the mutually exclusive userspace-only `IP_CT_NEW_REPLY` arm is absent.

Source evidence: `vendor/linux/include/uapi/linux/netfilter/nf_conntrack_common.h:1-168`; direct kernel inclusion: `vendor/linux/include/linux/netfilter/nf_conntrack_common.h:1-34`; direct use context: `vendor/linux/net/netfilter/nf_conntrack_core.c:1909-1919,2743,2788-2789`.
