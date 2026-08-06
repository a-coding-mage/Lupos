# Rust review — S014258

Reviewed `src/include/linux/lsm/apparmor.rs` against pinned
`vendor/linux/include/linux/lsm/apparmor.h` and the frozen x86_64/AArch64
configuration records.

## Result

No Rust-specific findings.

`CONFIG_SECURITY_APPARMOR` is disabled in both frozen configurations.  The C
definition of `struct lsm_prop_apparmor` therefore has no members.  The Rust
`#[repr(C)] pub struct lsm_prop_apparmor {}` is a zero-sized, alignment-one
representation, matching the selected empty C aggregate and preserving its
zero-sized embedded-field role in `struct lsm_prop`.  It introduces no Rust
references, raw-pointer dereferences, ownership, `Drop`, `Send`/`Sync`, or
unsafe-boundary concerns.  The conditional pointer member is correctly absent
for the frozen configuration union.

No compiler, formatter, rust-analyzer diagnostic, build, or test command was
used.
