# Rust review — S016353 (slot 2)

## Verdict

ACCEPT — no Rust-specific finding.

## Evidence reviewed

- Complete pinned `vendor/linux/include/uapi/linux/reboot.h` at revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, which matches
  `vendor/linux.SHA`.
- Fresh candidate `src/include/uapi/linux/reboot.rs`, task evidence, and the
  S016353 queue/scope/symbol records for the common x86_64/AArch64 scope.
- Pinned immediate semantic consumers: `kernel/reboot.c:728-859` and
  `kernel/pid_namespace.c:322-342`.

## Rust and ABI audit

- The source has thirteen object-like UAPI integer macros and no structure,
  union, enum, function declaration, externally linked object, mutable state,
  ownership transfer, or configuration-selected definition other than its
  include guard.  The candidate exports exactly those thirteen names, with the
  exact bit values and no invented declarations.
- The seven values representable as a C `int` retain their `i32` category:
  `LINUX_REBOOT_MAGIC2`, `LINUX_REBOOT_MAGIC2A`, `LINUX_REBOOT_MAGIC2B`,
  `LINUX_REBOOT_MAGIC2C`, `LINUX_REBOOT_CMD_RESTART`,
  `LINUX_REBOOT_CMD_CAD_OFF`, and `LINUX_REBOOT_CMD_KEXEC` are all within the
  signed 32-bit range.  This includes preserving a signed integer constant
  expression for the low hexadecimal literals.
- `LINUX_REBOOT_MAGIC1` and the six high command literals do not fit C
  `int`; their unsuffixed hexadecimal spelling has `unsigned int` type on
  both frozen targets.  Their `u32` candidate types preserve that 32-bit
  unsigned literal category and every bit exactly.  In particular, the
  source's `int` magic parameters in `kernel/reboot.c` are compared under C's
  usual arithmetic conversions, while `cmd` is `unsigned int`; the Rust
  translation will need explicit call-boundary conversions rather than a
  lossy type change in this UAPI definition.
- There is no aggregate layout, calling convention, pointer, string-literal,
  atomic, aliasing, pinning, synchronization, allocation, `Drop`, panic, or
  `unsafe` behavior to audit in this declarative header.  The candidate
  introduces none of those constructs, no placeholder, and no Rust test.
- SPDX and immutable source/revision/task provenance match the pinned header
  and S016353 mapping.  `architectures: common` correctly covers both frozen
  targets without adding a Rust `cfg` divergence.

No source files were edited. No compiler, formatter, build, test, or runtime
command was run.
