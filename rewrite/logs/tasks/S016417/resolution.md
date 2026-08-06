# S016417 applier resolution

Pinned source reopened: `vendor/linux/include/uapi/linux/thermal.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` (the revision recorded in
`vendor/linux.SHA`).  The complete header and its immediate consumers
`include/linux/thermal.h` and `drivers/thermal/thermal_netlink.c` were
rechecked for the common x86_64/aarch64 scope.

## Review dispositions

| Review finding | Disposition | Upstream evidence |
| --- | --- | --- |
| Parity review: no finding | Accepted after independent complete-header recheck. | `include/uapi/linux/thermal.h:1-110` |
| Rust review: no finding | Accepted after ABI/storage and UAPI-consumer recheck. | `include/uapi/linux/thermal.h:9-108`; `include/linux/thermal.h:75,103`; `drivers/thermal/thermal_netlink.c:18-23,906-914` |

## Final source and ABI disposition

- The source has six unconditionally defined enum tags.  The candidate retains
  each as a distinct `#[repr(transparent)]` wrapper over frozen-target C
  `int`, preserving the required 32-bit integer storage/call representation
  while admitting received netlink values that are not named enumerators.
  Every source enumerator is present with its sequential source value:
  device mode `0..1`, trip type `0..3`, attributes `0..28`, sampling `0..1`,
  events `0..20`, and commands `0..11`.
- Each derived UAPI maximum preserves its source expression and value:
  attribute `27`, sampling `0`, event `19`, and command `10`.  The three
  non-string numeric macros preserve C `int` values `20`, `0x1`, `0x2`, and
  `0x02`; the latter is the generic-netlink family version.
- The three string-literal macros preserve exact ASCII bytes and one terminal
  NUL in immutable static `c_char` arrays: `"thermal"` (8 bytes),
  `"sampling"` (9 bytes), and `"event"` (6 bytes).  This supplies static
  string storage and pointer decay through `.as_ptr()` without ownership or
  mutability changes.  `thermal_netlink.c:18-23,906-914` confirms these feed
  generic-netlink group/family names, while the numeric values feed the policy,
  operation, and reservation bounds.
- The only conditional is the C multiple-inclusion guard, which has no Rust
  module counterpart.  There is no configuration or architecture branch; the
  immutable provenance exactly identifies the UAPI source, pinned revision,
  `common` architecture scope, and task ID.  The syscall-note SPDX identifier
  is retained exactly.  No branding change exists.
- This declarative header creates no allocation, ownership transfer, borrow,
  lock, RCU, refcount, callback, cleanup, or unsafe boundary.  Its outstanding
  Phase-0 semantic categories therefore resolve as: lifetime/ownership,
  locking/RCU, and refcounting `NOT_APPLICABLE`; ABI is the explicit
  transparent C-`int` representation and static NUL-terminated character
  storage described above; semantic dependencies are limited to the preserved
  integer and string values.

No source edit was warranted.  No compiler, formatter, linker, test, emulator,
debugger, benchmark, or runtime command was run.
