# Rust review — S014172 (slot 2)

## Verdict

REJECT — the `KERN_*` string macros must not be represented as Rust `&str`
constants.  The replacement must preserve the source macros' C string-literal
and call-boundary semantics before this task can be accepted.

## Evidence reviewed

- Complete pinned `vendor/linux/include/linux/kern_levels.h` at revision
  `425f94c2954b1fe80ebdbf9b29854e89750355df` (the revision recorded by this
  task's provenance and `vendor/linux.SHA`).
- Candidate `src/include/linux/kern_levels.rs`, the task's implementation and
  candidate evidence, and the selected x86_64/AArch64 symbol inventory.
- Representative pinned consumers: `kernel/events/core.c:563`,
  `kernel/locking/lockdep.c:725-783`, `kernel/module/main.c:3963`, and
  `include/drm/drm_print.h:524,549`.

No source, manifest, or queue file was edited by this reviewer. No compiler,
formatter, test, or runtime command was run.

## Finding

1. **High — `&str` does not preserve C string-literal macro semantics or the
   C string call boundary.**

   At `include/linux/kern_levels.h:5,8-17,24`, each `KERN_SOH`,
   `KERN_EMERG` through `KERN_DEBUG`, `KERN_DEFAULT`, and `KERN_CONT` macro
   expands to C string-literal tokens.  Thus each resulting literal is a
   NUL-terminated character array which decays to a one-word `const char *`
   in calls.  The macro token form also deliberately supports adjacent-literal
   concatenation, for example `printk(KERN_WARNING "...")` in
   `kernel/events/core.c:563` and `printk(KERN_DEFAULT "Modules linked in:")`
   in `kernel/module/main.c:3963`; passing the macro itself as a C string is
   also required by `drm_dev_printk(..., KERN_ERR, ...)` at
   `include/drm/drm_print.h:524`.

   The candidate's `&str` constants at
   `src/include/linux/kern_levels.rs:8,13-23,30` exclude the final NUL and are
   two-word UTF-8 slice references, not character arrays or C pointers.  They
   cannot be supplied to a C `%s`/`const char *` boundary without an additional
   conversion, and they cannot participate in the source's token-level
   adjacent-literal construction.  Consequently the claim that they preserve
   the "same byte sequences" is incomplete: every C literal includes its
   terminating zero and has array/decay behavior absent from `&str`.

   Represent these definitions with static NUL-terminated byte storage and a
   controlled C-character-pointer view at FFI/call boundaries, together with
   a faithful strategy for the translated call sites' compile-time prefix
   construction.  Do not expose a Rust `&str` as the direct equivalent of the
   C macro.  This applies equally to the empty `KERN_DEFAULT`, whose source
   literal still consists of a terminating NUL.

## Checks that passed

- `KERN_SOH_ASCII` is correctly represented as `core::ffi::c_int = 1`:
  the source character constant has C `int` expression type and value 1.
- `LOGLEVEL_SCHED` through `LOGLEVEL_DEBUG` are all signed C `int` constant
  expressions in the pinned header; their values and the candidate's
  `core::ffi::c_int` representation match on the frozen targets.
- The header has no structs, unions, mutable storage, ownership transfer,
  locking, allocation, panic path, unsafe code, or configuration-specific
  declaration other than its include guard.  The candidate adds no Rust test
  or placeholder and carries the required immutable provenance.

