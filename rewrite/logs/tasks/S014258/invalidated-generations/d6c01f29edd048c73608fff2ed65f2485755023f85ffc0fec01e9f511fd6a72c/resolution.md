# Resolution — S014258

Reviewed the complete pinned `vendor/linux/include/linux/lsm/apparmor.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 and AArch64
configurations, the candidate, and both independent reports.

## Disposition

- Parity review: accepted. Both frozen configurations contain
  `# CONFIG_SECURITY_APPARMOR is not set`; therefore the `#ifdef
  CONFIG_SECURITY_APPARMOR` region excludes the sole `struct aa_label *label`
  member on every approved target. The remaining C aggregate has no members,
  exactly represented by `#[repr(C)] pub struct lsm_prop_apparmor {}`.
- Rust review: accepted. The selected aggregate is zero-sized and has no
  ownership, pointer, aliasing, drop, synchronization, unsafe, or FFI member
  contract. `#[repr(C)]` preserves its C aggregate representation at the
  `struct lsm_prop::apparmor` embedding site in `include/linux/security.h`.
- Header guard and forward declaration: no Rust runtime declaration is
  required for the selected configuration. The include guard is
  preprocessing-only; the forward-declared `aa_label` is used solely by the
  excluded member.
- Protocol correction applied: the destination provenance SPDX identifier is
  `GPL-2.0-only`, as required for every fresh Rust translation. No semantic
  source change was needed.

## Semantic closure

For both x86_64 and AArch64, the selected conditional branch is the disabled
AppArmor branch; `struct lsm_prop_apparmor` has no fields, storage ownership,
lifetime, locking/RCU/refcount, linkage, alignment-sensitive member, or
exported ABI contract beyond its `#[repr(C)]` empty-aggregate representation.
All task-local `PENDING_REVIEW` semantic items are resolved by this evidence:
the include-guard directives are preprocessing-only, the AppArmor conditional
is false in both frozen configurations, and the type's selected layout and
lifetime contracts are as stated above.

No compiler, formatter, rust-analyzer diagnostic, build, link, test, runtime,
or debugger command was used.
