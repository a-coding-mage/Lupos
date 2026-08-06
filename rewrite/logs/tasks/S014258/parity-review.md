# Parity review — S014258

Reviewed `src/include/linux/lsm/apparmor.rs` against the complete pinned
`vendor/linux/include/linux/lsm/apparmor.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Result: PASS — no parity findings.

- The SPDX identifier and immutable provenance identify the assigned Linux
  header, revision, common architecture scope, and task ID.
- The only operative declaration is `struct lsm_prop_apparmor`.  In both
  frozen configurations `CONFIG_SECURITY_APPARMOR` is unset, so the C
  conditional removes its sole `label` member.  The Rust `#[repr(C)]`
  empty `lsm_prop_apparmor` preserves the resulting memberless type for the
  approved x86_64/aarch64 configuration union.
- The source context embeds this type as `apparmor` in `struct lsm_prop`
  (`include/linux/security.h`); no additional symbols, state, side effects,
  linkage, cleanup, or synchronization behavior are present in the assigned
  header.
- Include guards are preprocessing-only and require no runtime Rust
  counterpart for the frozen translation unit.

Manual source inspection only; no compiler, formatter, rust-analyzer,
build, test, debugger, or runtime tool was used.
