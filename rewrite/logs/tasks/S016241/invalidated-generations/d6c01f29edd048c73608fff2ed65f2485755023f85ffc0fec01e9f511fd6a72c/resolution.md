# Applier resolution — S016241

Reopened the complete pinned source
`vendor/linux/include/uapi/linux/membarrier.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the fresh candidate, and both
independent review reports.

## Finding dispositions

Both reviews reported no findings. Independent application confirms that no
source amendment is required:

- `enum membarrier_cmd` is declaration-only UAPI.  The candidate exposes its
  C `int` value representation as `membarrier_cmd = core::ffi::c_int` and
  retains every enumerator exactly: `QUERY = 0`; bits 0 through 9 for the ten
  command values; and `SHARED = GLOBAL` for the compatibility alias.
- `enum membarrier_cmd_flag` is likewise a C-`int` declaration-only enum.  Its
  sole enumerator, `MEMBARRIER_CMD_FLAG_CPU`, remains bit 0.
- The only conditional and operative macro in the pinned header are the
  conventional `_UAPI_LINUX_MEMBARRIER_H` include guard.  It controls repeated
  C-header inclusion but declares no UAPI value; Rust module loading supplies
  the equivalent one-definition behavior, so no exported Rust item is needed.
- The header contains no functions, object storage, layout-bearing aggregate,
  pointer, ownership, allocation, locking, cleanup, or architecture/configuration
  branch.  Its ABI/lifetime records are therefore closed for this task as
  declaration-only C-`int` constants with no ownership or lifetime contract.

The required immutable provenance identifies the exact pinned source, revision,
common architecture scope, and task.  The upstream copyright and permission
notice are retained, all public UAPI names remain unchanged, and the candidate
contains no prohibited placeholder or Rust test configuration.

## Result

Accepted unchanged.  No unresolved source, ABI, lifetime, locking, or branding
question remains for S016241.
