# Resolution — S016252

Applier reopened the complete pinned source
`vendor/linux/include/uapi/linux/mptcp_pm.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the candidate, both independent
review reports, and the consuming initializer in
`vendor/linux/net/mptcp/pm_netlink.c:631`.

## R1 — MPTCP_PM_NAME pointer decay

**Accepted and fixed.**  The C replacement list is the string literal
`"mptcp_pm"`; in the `struct genl_family` initializer it undergoes the normal
array-to-pointer conversion for `.name`.  The Rust translation now keeps the
exact immutable nine-`c_char` (including NUL) static backing storage private
and exposes `MPTCP_PM_NAME` as a `*const c_char` constant obtained from that
storage.  Its pointer is valid for the complete program lifetime and no mutable
access is exposed.  This is the macro-equivalent expression form required by
the source initializer.

## R2 — named enum tags

**Accepted and fixed.**  `enum mptcp_event_type` and
`enum mptcp_event_attr` are now distinct `#[repr(transparent)]` newtypes over
`c_int`; therefore each preserves its independent C tag and selected ABI while
its public field accepts every `c_int` bit pattern, including netlink values
outside the listed enumerators.  Their respective constants, including
`__MPTCP_ATTR_MAX` and `MPTCP_ATTR_MAX`, have the corresponding distinct
newtype.  The anonymous C enum members remain `c_int` constants, matching
their untagged C declarations.

## Final source check

All upstream values, intentional numbering gaps, anonymous-enum sentinels and
derived maxima remain unchanged.  The header has no selected conditional
behavior, functions, mutable storage, locking, ownership transfer, or FFI
exports.  The task's semantic records are resolved by the source facts above:
the named enums have transparent `c_int` layout/alignment and no ownership or
lifetime beyond ordinary by-value integers; the string macro has immutable
static backing storage and a process-lifetime pointer; all anonymous enums and
integer macros are by-value `c_int` constants with no ownership, locking, RCU,
or refcount contract.

No build, compiler, formatter, test, linker, debugger, emulator, or runtime
command was run.
