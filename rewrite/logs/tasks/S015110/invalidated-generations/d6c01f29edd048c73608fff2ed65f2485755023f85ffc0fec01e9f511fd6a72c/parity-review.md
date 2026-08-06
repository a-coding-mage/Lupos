# Parity review — S015110

Reviewed: 2026-08-05T23:39:12.612Z  
Reviewer: P01 slot 1 (parity)  
Pinned source: `vendor/linux/include/linux/sunrpc/xprtrdma.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`

## Result

Accepted. No parity findings.

## Evidence checked

- The candidate provenance identifies the exact source path, frozen revision, `common` architecture membership, and task ID. It retains the upstream dual-license SPDX identifier and Network Appliance copyright notice.
- All six selected macros are present with their exact values and C literal-width intent: the three slot-table values are `u32` (`4`, `128`, `16384`) matching `U`-suffixed `unsigned int` macros; the three inline thresholds are signed `i32` (`1024`, `4096`, `65536`) matching the unsuffixed `int` macros.
- `rpcrdma_memreg` is public and `#[repr(C)]`. Its eight discriminants retain exact source order and values: `BOUNCEBUFFERS=0`, then `REGISTER=1`, `MEMWINDOWS=2`, `MEMWINDOWS_ASYNC=3`, `MTHCAFMR=4`, `FRWR=5`, `ALLPHYSICAL=6`, and sentinel `LAST=7`.
- The header has no configuration conditional around these symbols. Both frozen configurations select `CONFIG_SUNRPC=y`; the Phase 0 symbol inventory selects this same unconditional six-constant/one-enum surface for `x86_64` and `aarch64`.
- Pinned RDMA transport consumers use the constants as scalar `unsigned int` initializers/range bounds and the enum values numerically. The candidate preserves those values, numeric ordering, and C ABI representation; no behavior, branch, or selected declaration is omitted.

No build, formatter, compiler, or runtime command was run.
