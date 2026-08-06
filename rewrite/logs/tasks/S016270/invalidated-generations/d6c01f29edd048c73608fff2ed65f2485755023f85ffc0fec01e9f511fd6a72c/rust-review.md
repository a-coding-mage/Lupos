# Rust review — S016270

Reviewer role: `rust_reviewer` (slot 2)  
Model: `gpt-5.6-terra`  
Reasoning effort: `high`

Reviewed only the fresh candidate at
`src/include/uapi/linux/netfilter/nf_conntrack_common.rs` against the pinned
`include/uapi/linux/netfilter/nf_conntrack_common.h`. No implementation or
parity-review artifact was read; no source was edited, built, formatted, or
tested.

## Findings

1. **High — SPDX provenance is not retained exactly.**  Upstream line 1 is
   `GPL-2.0 WITH Linux-syscall-note`, while candidate line 1 says
   `GPL-2.0-only`. This is an unauthorized change to the UAPI source's SPDX
   identifier. Restore the upstream expression exactly.

2. **High — the candidate erases `__KERNEL__` conditional API selection.**
   The source selects `IP_CT_NEW_REPLY` for non-kernel consumers (lines 30–34)
   but `IP_CT_UNTRACKED` only for kernel consumers. The candidate omits
   `IP_CT_NEW_REPLY` and exports `IP_CT_UNTRACKED` unconditionally (lines
   23–24). It likewise unconditionally exports the kernel-only
   `IPS_NAT_CLASH_BIT`/`IPS_NAT_CLASH` (source lines 100–107), `__IPCT_MAX`
   (147–149), and `NF_CT_EXPECT_DEAD`/`NF_CT_EXPECT_MASK` (162–166).
   The Rust mapping must explicitly model the selected kernel/non-kernel
   boundary, or document and mechanically enforce a kernel-only Rust module;
   it cannot silently present kernel-only names as UAPI names nor omit the
   userspace compatibility enumerator.

3. **Medium — enum ABI/type decision is asserted without the required ABI
   evidence.**  Candidate lines 13–15 state that all four C enum tags have an
   `int` representation and replace them with `i32` aliases. The frozen ABI
   records for these enum types remain `PENDING_REVIEW`, and a Rust type alias
   supplies neither a distinct C enum type nor an explicit FFI representation.
   Establish the selected compiler ABI for both architectures and the intended
   boundary usage, then record it in `rewrite/ABI.tsv`; use a representation
   that preserves that established contract. This must also settle whether
   arbitrary integer-valued enum objects are intentionally accepted at Rust
   call sites.

4. **Medium — `NF_CT_STATE_BIT` changes the invalid-input failure model.**
   C macro expansion uses C signed remainder and a signed shift (source line
   38); a malformed negative `ctinfo` can make the shift count invalid, for
   which C has undefined behavior. Candidate lines 28–30 expose a safe Rust
   `const fn` accepting every `i32`; its shift failure is Rust
   panic/check behavior and can differ by build configuration. Either prove
   all reachable inputs satisfy the C macro's shift precondition, or encode
   the corresponding caller contract/unsafe boundary without introducing a
   new safe, panic-capable UAPI operation.

## Positive checks

For the common enumerators and bit values, the candidate's explicit values and
derived masks match the source. There are no aggregate layouts, pointer
ownership rules, atomics, or `unsafe` blocks in this header candidate.

## Disposition

Changes are required before source acceptance. The applier must resolve every
finding with pinned-source and frozen-ABI evidence.
