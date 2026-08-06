# S015284 resolution

Role: `applier`
Model: `gpt-5.6-terra`
Reasoning effort: `high`

This resolution was made by source inspection only. No compiler, formatter,
linker, test, runtime, or compiler-backed diagnostic was invoked.

## P1 / R1 — resolved: C string-literal representation

`vendor/linux/include/linux/uts.h:9`, `:13`, and `:17` define object-like
macros whose selected expansions are C string literals. The direct pinned
consumer `vendor/linux/init/version-timestamp.c:10-21` aggregate-initializes
the fixed `char[__NEW_UTS_LEN + 1]` members of `struct new_utsname` with those
expansions. The candidate's `&str` values were therefore not equivalent:
they were fat references and omitted the C terminating NUL.

The operative Rust constants are now fixed byte arrays with the literal C
bytes and their terminating NULs:

- `UTS_SYSNAME: [u8; 6] = *b"Linux\\0"`
- `UTS_NODENAME: [u8; 7] = *b"(none)\\0"`
- `UTS_DOMAINNAME: [u8; 7] = *b"(none)\\0"`

They are array values, not slices, pointers, strings, conversion helpers, or
a substitute initialization API. This permits the future path-preserving
translation of `init/version-timestamp.c` to use the exact literal array
payload when initializing its corresponding fixed UTS-name fields.

## R2 — resolved: selected `#ifndef` behavior

The outer `_LINUX_UTS_H` `#ifndef`/`#define`/`#endif` at lines 2-20 is a C
preprocessor include guard. It has no runtime object, linkage, layout,
lifetime, or C-to-Rust data counterpart; the path-preserving Rust module is
included by the Rust module graph rather than through textual C inclusion.

Each `UTS_*` default is independently protected by `#ifndef`/`#endif` at
lines 8-10, 12-14, and 16-18. The frozen x86_64 and AArch64 header-consumer
commands in `rewrite/FILE_MAP.tsv` for `init/version.o` contain no
`-DUTS_SYSNAME`, `-DUTS_NODENAME`, or `-DUTS_DOMAINNAME`; each condition is
therefore true in the approved configuration union. The frozen configurations
set `CONFIG_DEFAULT_HOSTNAME="(none)"`, selecting the recorded nodename bytes.
The conditional definitions consequently select exactly the three arrays
above for both approved architectures.

The pinned source has UTS predefinition mechanisms only in non-approved
architectural Makefiles (for example `arch/nios2/Makefile:18,36` and
`arch/m68k/Makefile:70`), not in the frozen x86_64/AArch64 build interface.
No alternate UTS override is selected or supported by this frozen task.
Representing an arbitrary C token definition before a textual include as a
Rust runtime/configuration facade would not preserve the selected source
contract; it is deliberately not introduced.

## R3 — task semantic facts closed

For both `x86_64` and `aarch64`, the S015284 rows in `rewrite/SYMBOLS.tsv`
are resolved as follows:

- `ifndef@2`, `_LINUX_UTS_H`, and `endif@20`: C textual include-guard only;
  no Rust value, ABI, lifetime, or synchronization contract.
- `ifndef@8`/`endif@10`, `ifndef@12`/`endif@14`, and
  `ifndef@16`/`endif@18`: true/default branches under the frozen command
  contexts; no selected override branch.
- `UTS_SYSNAME`, `UTS_NODENAME`, and `UTS_DOMAINNAME`: compile-time array
  values with the exact literal bytes above; no storage, linkage, exported
  symbol, ownership, allocation, locking, RCU, refcounting, or runtime
  lifetime.

`uts.h` declares no ABI-bearing type, function, static/global storage, or
FFI item. Accordingly, its empty S015284 ABI and lifetime inventories remain
not applicable rather than incomplete. The source provenance remains the
pinned `425f94c2954b1fe80ebdbf9b29854e89750355df` revision and common
architecture scope. The branding allowlist is empty; `Linux` is retained.

All review findings are resolved from the pinned source and frozen Phase 0
evidence. No finding remains open.
