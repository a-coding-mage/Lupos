# Resolution — S016271

Applier: `gpt-5.6-terra`, high reasoning effort.

## Source recheck

I reopened the complete pinned UAPI declaration
`vendor/linux/include/uapi/linux/netfilter/nf_conntrack_ftp.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its in-kernel wrapper
`include/linux/netfilter/nf_conntrack_ftp.h`, and both selected x86_64
consumers, `net/netfilter/nf_conntrack_ftp.c` and `net/netfilter/nf_nat_ftp.c`.
The declaration has four implicit consecutive enumerators (`0` through `3`),
is stored in `struct ftp_search`, and crosses the NAT-hook function interface
by value.

## Review dispositions

| Finding | Disposition | Evidence |
| --- | --- | --- |
| Parity review acceptance of a transparent `c_int` wrapper | Rejected | The header identifies an enum tag but specifies neither its compatible integer type nor its layout or calling ABI. Its four enumerator constants do not establish enum-object representation. |
| R1: enum-compatible type is not established | Accepted; task BLOCKED | The frozen ABI record for `enum nf_ct_ftp_type` remains `PENDING_REVIEW`. The frozen x86_64 compile commands and Phase 0 identity identify LLVM 19.1.7, target, and flags, but contain no recorded enum size, alignment, signedness, compatible integer type, or by-value ABI for this declaration. No frozen compiler-predicate probe covers it. |

The candidate comment that C ABI is `int` and its use of `core::ffi::c_int`
are therefore unsupported assumptions. No replacement representation can be
selected exactly from the frozen evidence. The task is blocked pending an
approved Phase 0 ABI extraction/review that captures this enum's frozen x86_64
representation and by-value convention. No compiler, formatter, linker, test,
or runtime command was used during this application review.
