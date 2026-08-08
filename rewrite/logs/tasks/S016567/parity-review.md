# Parity review — S016567 (attempt 2, slot 1)

**Result: APPROVE**

Reviewed only the pinned Linux header `include/xen/interface/features.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen S016567 scope/symbol
rows, the current candidate `src/include/xen/interface/features.rs`, and direct
local Xen consumers.

## Source comparison

- Linux symbols `XENFEAT_writable_page_tables` through `XENFEAT_dom0` are
  active numeric macros with values 0 through 11. The candidate exports each
  with the same `i32` value. Local consumer `include/xen/features.h` passes
  these feature indices to `xen_feature(int flag)`, which is consistent with
  the candidate's signed 32-bit integer representation.
- Linux symbol `XENFEAT_grant_map_identity` appears only inside the block
  comment at lines 66–70 of the pinned header; it is not a preprocessor macro.
  The candidate correctly does not define it. The frozen symbol row records
  the source location but does not change the pinned source's inactive
  preprocessor state.
- Linux symbols `XENFEAT_memory_op_vnode_supported`,
  `XENFEAT_ARM_SMCCC_supported`, `XENFEAT_linux_rsdp_unrestricted`,
  `XENFEAT_not_direct_mapped`, and `XENFEAT_direct_mapped` retain their active
  values 13, 14, 15, 16, and 17 respectively. Direct aarch64 consumer
  `include/xen/arm/swiotlb-xen.h` uses the last two as feature indices; the
  candidate preserves those indices.
- Linux symbol `XENFEAT_NR_SUBMAPS` remains the active value 1. Direct
  consumer `include/xen/features.h` uses it in `xen_features[XENFEAT_NR_SUBMAPS
  * 32]`; the candidate preserves the numeric constant and its index role.
- The Linux include guard introduces no runtime symbol, ABI layout, linkage,
  allocation, locking, error path, or conditional feature behavior. The Rust
  module file needs no corresponding runtime definition. No non-allowlisted
  branding is present.

No SC1 findings. This was a manual source inspection only; no compiler,
formatter, test, linker, runtime, or diagnostic tool was used.
