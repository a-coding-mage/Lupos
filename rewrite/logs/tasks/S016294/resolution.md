# S016294 applier resolution

Result: **BLOCKED**.  The candidate cannot be accepted and no source edit to
`xt_state.rs` can make this task correct while preserving its required
cross-header relationship.

## Review dispositions

1. **Parity review: rejected.**  Its assertion that upstream
   `IP_CT_NUMBER` is 4 (and therefore that `XT_STATE_UNTRACKED` is 32) is
   contradicted by the complete upstream `enum ip_conntrack_info` declaration
   in `include/uapi/linux/netfilter/nf_conntrack_common.h:7-35`.  The preceding
   enumerators have values `IP_CT_ESTABLISHED=0`, `IP_CT_RELATED=1`,
   `IP_CT_NEW=2`, `IP_CT_IS_REPLY=3`,
   `IP_CT_ESTABLISHED_REPLY=3`, and `IP_CT_RELATED_REPLY=4`; the following
   implicit `IP_CT_NUMBER` enumerator is therefore 5.  Consequently the
   pinned `xt_state.h:8` expression is `1 << (5 + 1)`, i.e. the C `int` value
   64, not 32.
2. **Rust review: rejected on the same source fact.**  The wrapper type,
   `c_int` macro-result representation, `#[repr(C)]` one-member
   `xt_state_info` layout, SPDX expression, and immutable provenance are all
   individually appropriate.  They do not cure the wrong imported enum value.

## Blocking dependency

`src/include/uapi/linux/netfilter/nf_conntrack_common.rs:39` currently defines
`IP_CT_NUMBER` as `ip_conntrack_info(4)`, contrary to the pinned source.  The
candidate's `XT_STATE_UNTRACKED` correctly retains a symbolic dependency on
that item, but consequently evaluates its required bit position from the
wrong value.  Replacing the expression in this task with a literal `6` or
`64` would conceal the dependency and would not be a faithful translation of
the upstream macro.

The already-closed dependency task `S016270` must be requeued and corrected
from its complete pinned source before this task can be resumed and reviewed
again.  This applier did not alter that other task's source, manifests, or
queue row.  No compiler, formatter, rust-analyzer, linker, test, debugger, or
runtime command was used.

## S016294 semantic-record closure

- `XT_STATE_BIT`: retained source expression is a C-`int` shift result whose
  valid-shift precondition remains explicit on the Rust entry point; the
  `ip_conntrack_info` ABI wrapper is the necessary operand representation.
- `XT_STATE_INVALID`: C `int` value `1 << 0`, therefore 1.
- `XT_STATE_UNTRACKED`: source relation is `1 << (IP_CT_NUMBER + 1)` and is
  blocked pending correction of the imported `IP_CT_NUMBER=5` definition.
- `struct xt_state_info`: one `unsigned int statemask` member; the candidate's
  `#[repr(C)] c_uint` field preserves the frozen x86_64 UAPI field type,
  four-byte size/alignment, and contains no ownership, pointer, packing, or
  bitfield concern.
- Header guard: preprocessing-only and intentionally has no Rust ABI item.
- License/provenance: exact `GPL-2.0 WITH Linux-syscall-note`, pinned source,
  revision, architecture, and task identifiers are present; no branding delta
  is involved.

The unresolved cross-task enum value prevents the required final mapping for
the `XT_STATE_UNTRACKED` record, so `S016294` must not transition to `DONE`.
