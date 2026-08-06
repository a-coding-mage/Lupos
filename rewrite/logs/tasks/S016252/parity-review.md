# Parity review — S016252 (slot 1)

Reviewed the complete pinned source `vendor/linux/include/uapi/linux/mptcp_pm.h`
at revision `425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/mptcp_pm.rs` for the frozen common x86_64/AArch64
scope.

## Result

No parity findings.

## Evidence checked

- The candidate retains the exact upstream SPDX expression and immutable
  provenance: source path, frozen revision, `common` architecture scope, and
  task ID `S016252`.
- `MPTCP_PM_NAME` is the exact nine-byte C character array for `"mptcp_pm"`,
  including its terminating NUL, with `c_char` elements; `MPTCP_PM_VER` remains
  the C-`int` value `1`.
- Both named enum tags, `mptcp_event_type` and `mptcp_event_attr`, are exposed
  as C-`int` ABI aliases. Every enumerator is present with its upstream value,
  including the intentional event numbering gaps: announced/removal `6/7`,
  subflow-established/closed `10/11`, subflow-priority `13`, and listener
  created/closed `15/16`.
- All four anonymous attribute namespaces and the command namespace preserve
  their complete source-order members, explicit `MPTCP_PM_ENDPOINT_ADDR = 1`,
  each private `__...MAX` sentinel, and every public `...MAX = __...MAX - 1`
  expression. Evaluated maxima are respectively 7, 11, 1, 6, 18, and 11.
- The header contains only the conventional `_UAPI_LINUX_MPTCP_PM_H` include
  guard. It has no Kconfig, architecture, feature, or other selected
  conditional declaration, and no structs, unions, functions, storage
  definitions, linkage declarations, locking, cleanup, or driver behavior.
  The candidate introduces none of these, nor a branding delta, test, stub,
  panic, or unsafe code.

No source, manifest, or non-review evidence file was modified by this
reviewer. No build, compiler, formatter, test, linker, debugger, emulator, or
runtime command was run.
