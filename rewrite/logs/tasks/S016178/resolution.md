# Resolution — S016178 / P02 / attempt 1

## Outcome: BLOCKED

The fresh candidate remains unchanged. The findings cannot be resolved from the permitted pinned source and frozen records without choosing a Rust UAPI/FFI contract that the evidence does not establish. No compiler, formatter, analyzer, test, runtime command, or historical Lupos source was used.

## Review-finding dispositions

### P1 — C enumerator namespace and integer behavior: accepted

`include/uapi/linux/if_vlan.h:21-48` declares three C enums and introduces their enumerators as bare C integer constants. The candidate instead supplies scoped Rust enum variants. The direct selected consumers require ordinary integer constants: `net/8021q/vlan.c:521-615` switches on the command enumerators and compares `args.u.name_type` to `VLAN_NAME_TYPE_HIGHEST`, while `net/8021q/vlan_dev.c:224-231` combines `VLAN_FLAG_*` with `u32` bit masks.

The source proves the listed values, but the frozen ABI entries for all three named enum tags on x86_64 and aarch64 retain `layout`, `alignment`, and `export_kind` as `PENDING_REVIEW`. It therefore does not establish either a two-target Rust representation for the named enum types or a semantics-preserving unscoped C-integer-constant interface. Inventing one would be a new UAPI contract, so this finding blocks the task.

### P1 — `vlan_ioctl_args` / union ABI and lifetime contract: accepted

The header has `struct vlan_ioctl_args` at lines 50-64 and a union at lines 54-61. The selected ioctl path copies that aggregate to and from userspace with `copy_from_user` / `copy_to_user` at `net/8021q/vlan.c:512-513` and `600-611`. Although the candidate visibly follows the field order and uses `#[repr(C)]`, all x86_64/aarch64 ABI rows for the struct and union retain `layout`, `alignment`, and `export_kind` as `PENDING_REVIEW`, and the matching lifetime rows retain their lifetime/ownership/locking fields pending. Those records do not supply the source-backed cross-language aggregate/union ABI and active-member contract required to close the UAPI mapping. This cannot be completed by appearance alone.

### P1 — `char[24]` representation: accepted

The header declares `char device1[24]` and `char device2[24]` at lines 52 and 55. The candidate instead exposes `[i8; 24]`. The reviewed selected ioctl path writes a terminator to both arrays (`vlan.c:515-517`) and transfers the whole aggregate across the user boundary (`vlan.c:512-513`, `600-611`). The frozen ABI/lifetime records do not close the selected-target plain-`char` contract; the candidate's signed-byte choice is consequently not source-proven for the required two-target UAPI boundary. No permissible evidence establishes that changing the signed interpretation is exact.

### P2 — include guard mapping: accepted

`_UAPI_LINUX_IF_VLAN_H_` is the operative preprocessor guard in `if_vlan.h:14-15,66` and is selected for both architectures in `SYMBOLS.tsv`. Its selection expression/status remain pending. The candidate provides no source-proven mapping of this C include-time contract to Rust module loading. Accepting the omission would require an unrecorded semantic decision.

### R1 — named C-enum ABI and value-domain substitution: accepted

The Rust review independently identifies the same unresolved type contract: the `cmd` field is explicitly an `int` in the source (`if_vlan.h:51`), whereas the separately named C enum tags are not used there to prove their Rust representation or their full FFI value domain. The six named-enum ABI `layout` records cited by the reviewer remain `PENDING_REVIEW` in `ABI.tsv`. The candidate's `#[repr(i32)]` enums both narrow the value domain and replace the C constant namespace. Pinpointing the enumerator values does not establish that substitution, so the review blocker is sustained.

## Frozen semantic closure

The proposal marks the affected ABI, symbol, and lifetime records `COMPLETE`, but the source review above disproves those proposed closures. In particular, the six enum-layout records identified in slot 2 cannot be finalized. The parity reviewer produced a finding report but no semantic-review TSV; no attestation was fabricated or retried. The task must remain blocked until a later permitted workflow can establish the missing two-target C ABI, plain-`char`, and preprocessor mapping evidence.
