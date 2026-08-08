# Implementation: S015088 attempt 1

Source: `vendor/linux/include/linux/sunrpc/gss_err.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The translation defines `OM_uint32` as `u32`, preserves every selected GSS
object-like macro as a public `OM_uint32` constant, and retains all six
function-like status macros as `macro_rules!` expressions.  The latter expand
their input once, matching the C expressions' single evaluation of `x`.

There are no configuration branches beyond the C include guard, no storage,
locking, allocation, ABI export, or lifetime behavior.  The frozen x86_64 and
AArch64 inventories select the same complete header definition set.

