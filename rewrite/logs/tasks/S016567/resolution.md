# Resolution — S016567

Resolved against the complete pinned
`vendor/linux/include/xen/interface/features.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`. This was source-only work; no
build, test, formatter, linker, or runtime command was run.

1. **Parity/Rust review: C integer category.** Accepted. Every active
   upstream replacement list is an unsuffixed decimal C `int` literal. The
   frozen AArch64 target has a 32-bit `int`, and consumers use feature flags
   as `int` values (including `xen_feature(int flag)`). All active Rust
   constants, including `XENFEAT_NR_SUBMAPS`, now use `i32`, preserving the
   signed C integer category. Any different translated use-site category must
   perform its conversion explicitly at that use site.

2. **Parity/Rust review: copyright notice.** Accepted. Restored the upstream
   `Copyright (c) 2006, Keir Fraser <keir@xensource.com>` notice immediately
   after the MIT SPDX line and before immutable provenance.

3. **Parity review: feature-contract commentary.** Accepted. Restored the
   public contracts for all active feature indices, including the conditional
   RSDP relaxation and the direct-map fallback assumptions. The deprecated
   `XENFEAT_grant_map_identity` apparent definition remains documentation only:
   it is in an upstream block comment and no Rust constant was added.

The final file retains each active source macro identifier and literal value:
indices 0 through 11, 13 through 17, and `XENFEAT_NR_SUBMAPS = 1`. It adds no
layout, linkage, storage, function, configuration branch, test, or inactive
feature definition.
