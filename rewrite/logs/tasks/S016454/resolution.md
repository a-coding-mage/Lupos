# Resolution — S016454

Applied source-only against `vendor/linux/include/uapi/linux/vesa.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`. The frozen queue row was verified
as `APPLYING` under `P02`, and the Phase 0 identity binds that same Linux
revision and both frozen configurations.

## Review finding dispositions

1. **Missing `VESA_BLANK_MAX` module export — resolved.**
   Added `pub const VESA_BLANK_MAX: i32` with the exact source expression's
   value, `VESA_POWERDOWN` (3). The associated constant remains consistent.

2. **C scalar copy semantics — resolved subject to the ABI blocker below.**
   Added `#[derive(Clone, Copy)]` to preserve ordinary by-value copying of the
   candidate scalar wrapper. This does not establish the wrapper's underlying
   ABI representation.

3. **SPDX mismatch — resolved.**
   Restored the exact upstream identifier:
   `GPL-2.0 WITH Linux-syscall-note`.

## Blocking ABI decision

The source declares the named C type `enum vesa_blank_mode`. Its values are
mechanically established as 0, 1, 2, and 3, and pinned consumers use the type
for statics and parameters. But the authoritative task rows in
`rewrite/ABI.tsv` for both `aarch64` and `x86_64` record its layout and
alignment as `PENDING_REVIEW`; the corresponding `rewrite/LIFETIMES.tsv` rows
also remain `PENDING_REVIEW`. The pinned header and frozen configuration/command
records inspected here do not mechanically establish the enum-compatible
integer type or its signedness. In particular, no task-specific ABI manifest
proves that the candidate's `i32` transparent wrapper matches the frozen C ABI
on both targets.

Per the rewrite protocol, that representation must not be guessed or closed as
an `i32` convenience choice. The task is therefore blocked pending authoritative
per-target enum layout/signedness evidence. No frozen manifest was edited.
