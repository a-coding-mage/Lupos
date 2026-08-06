# Rust review — S016271

Scope reviewed: `src/include/uapi/linux/netfilter/nf_conntrack_ftp.rs` against
`vendor/linux/include/uapi/linux/netfilter/nf_conntrack_ftp.h`, plus its two
selected in-kernel consumers and wrapper declaration.

## Finding R1 — enum compatible type is not established

Reject / BLOCKED pending Phase 0 ABI evidence.  The source has one UAPI enum
and no storage, pointer, ownership, byte-order, mask, or
configuration-dependent operation.  Its ABI record remains `PENDING_REVIEW`:
the C header does not specify the implementation-selected compatible integer
type, its size, alignment, signedness, or the by-value ABI.  C permits an enum
to be compatible with an implementation-defined signed or unsigned integer
type; the header's four small non-negative enumerators do not mechanically
prove `int`.

Consequently, `#[repr(transparent)]` over `core::ffi::c_int` is a new ABI
assumption, not a proven translation.  The wrapper correctly avoids Rust's
closed-value validity contract, but cannot establish that an `int` wrapper has
the source enum's representation in the frozen compiler configuration.  This
matters because the enum is stored in `ftp_search` and passed by value through
the NAT hook.  Do not accept or substitute a different integer type without
the required frozen ABI evidence.

The four typed constants otherwise retain the C source order and implicit
values `0`, `1`, `2`, and `3`.  There are no endian-qualified fields, masks,
shifts, signed arithmetic operations, or target-conditional branches to
translate.
The provenance header matches task S016271, the pinned Linux path and revision,
and the frozen `x86_64` architecture membership.

Manual source inspection only; no compiler, formatter, linker, test, or
compiler-backed diagnostic was invoked.
