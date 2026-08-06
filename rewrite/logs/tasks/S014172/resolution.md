# Resolution — S014172

Reviewed the complete pinned `vendor/linux/include/linux/kern_levels.h` at
revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, the fresh candidate, both
independent review reports, and the task inventory.

## Disposition

1. **P1 / Rust finding: accepted and fixed.**  `KERN_SOH`, all named
   `KERN_<LEVEL>` prefixes, `KERN_DEFAULT`, and `KERN_CONT` are now static,
   NUL-terminated `[c_char; N]` arrays.  Each has an explicit thin
   `*const c_char` `_PTR` view for the C-string call boundary.  The
   `kern_<level>_cstr!` macros accept the translated adjacent literal tokens,
   create the full prefix-plus-message-plus-one-NUL static literal with
   `concat!`, and return the equivalent C-character pointer.  This replaces
   the rejected `&str` representation; no `&str` remains authoritative.
2. **P2: accepted and fixed.**  The source SPDX line now exactly retains the
   pinned `GPL-2.0` identifier rather than substituting `GPL-2.0-only`.
3. **Verified mappings.**  `KERN_SOH_ASCII` remains C `int` value `1`; every
   `LOGLEVEL_*` definition remains its pinned signed C `int` value.  The file
   has no selected configuration branch beyond its C include guard and has no
   ownership, locking, RCU, refcount, allocation, ABI layout, or lifetime
   record to close.

All 50 S014172 `SYMBOLS.tsv` rows, covering the include guard and every
operative macro for both frozen architectures, are now `COMPLETE`; the
S014172 `SCOPE.tsv` semantic status is likewise `COMPLETE`.  This task has no
S014172 rows in `ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, or `BLOCKERS.tsv`.

The original implementation and candidate reports remain as the implementation
stage evidence.  This resolution records the final applier correction.  No
build, formatter, linker, test, runtime, or benchmark command was run.
