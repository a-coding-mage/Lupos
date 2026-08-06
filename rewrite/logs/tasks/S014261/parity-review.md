# Parity review — S014261

Reviewed `src/include/linux/lsm/smack.rs` against the complete pinned
`vendor/linux/include/linux/lsm/smack.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus its `struct lsm_prop`
embedding in `include/linux/security.h` and the frozen configuration evidence.

## Result

Accepted: no parity findings.

## Source evidence

- `struct lsm_prop_smack` is the sole concrete type in the task inventory.
- Its only C member, `struct smack_known *skp`, is entirely guarded by
  `#ifdef CONFIG_SECURITY_SMACK`.
- Both frozen configurations contain `# CONFIG_SECURITY_SMACK is not set`.
  The selected definition is consequently a memberless C struct.
- The candidate supplies exactly that selected representation as
  `#[repr(C)] pub struct lsm_prop_smack {}`, preserving the distinct memberless
  Smack property nested in `struct lsm_prop` without introducing unselected
  Smack storage or behavior.
- The `struct smack_known;` declaration has no selected use once the guarded
  pointer member is removed, so no opaque Rust declaration is required for
  this configuration union.

No source, queue, evidence, or toolchain finding requires a candidate change.
