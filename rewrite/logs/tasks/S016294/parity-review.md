# S016294 parity review (slot 1)

Scope: source-only comparison of `src/include/uapi/linux/netfilter/xt_state.rs`
with pinned `vendor/linux/include/uapi/linux/netfilter/xt_state.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, for the frozen x86_64 UAPI
context. No compiler, formatter, test, or runtime tool was used.

## Result

No parity findings.

## Checked source evidence

- The immutable provenance identifies the exact Linux header, pinned revision,
  x86_64 architecture, and task `S016294`; the SPDX expression is preserved as
  `GPL-2.0 WITH Linux-syscall-note`.
- `XT_STATE_BIT` retains the C expression `1 << ((ctinfo % IP_CT_IS_REPLY) +
  1)`, uses the distinct C-`int` conntrack-info representation from the
  required sibling UAPI context, and retains the C undefined-input shift
  precondition as an unsafe caller contract.
- `XT_STATE_INVALID` remains the C `int` expression `1 << 0`; in the frozen
  x86_64 context it is 1. `XT_STATE_UNTRACKED` remains `1 << (IP_CT_NUMBER +
  1)`; the required conntrack enum fixes `IP_CT_NUMBER` at 4, yielding 32.
- `xt_state_info` is `#[repr(C)]` with exactly one `c_uint` `statemask` field,
  preserving C `unsigned int` width/alignment and the header's 4-byte x86_64
  UAPI layout.

No omission, changed value, changed enum relation, layout deviation, or
unallowlisted branding was found.
