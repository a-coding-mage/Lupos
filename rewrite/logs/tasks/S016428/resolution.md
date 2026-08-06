# Resolution: S016428

Pinned source: `vendor/linux/include/uapi/linux/tty_flags.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## P1 / R1 — resolved

The frozen x86_64 and AArch64 kernel translations correspond to upstream
preprocessing with `__KERNEL__` defined.  Consequently, the Rust kernel
surface now omits exactly the 19 names declared in upstream's two `#ifndef
__KERNEL__` blocks: the ten `ASYNCB_*` names at lines 43--52 and the nine
`ASYNC_*` names at lines 86--94.  This resolves both reviewers' finding that
the candidate exported names absent from the original kernel surface.

`ASYNC_SUSPENDED` remains because upstream declares it outside those blocks at
line 57.  Its UAPI expression uses the excluded `ASYNCB_SUSPENDED` operand;
the Rust constant records the same unsigned 32-bit UAPI value (`1U << 30`)
without re-exporting that guarded operand.  All other retained constants keep
their upstream spelling, source-order relationships, `c_int` bit-position
type, or `u32` `1U`-derived type as applicable.

No branding delta, ABI item, ownership/lifetime rule, unsafe operation, test,
formatter, compiler, or runtime action was introduced.
