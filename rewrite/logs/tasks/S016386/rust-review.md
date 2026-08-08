# Rust source review — S016386 / attempt 1 / P01

Reviewer: `rust_reviewer` (`gpt-5.6-terra`, high).  This review used only the
pinned UAPI header, the Rust candidate and candidate snapshot, frozen task
records, and pinned UAPI consumer contexts.  No compiler, formatter, test,
analyzer, historical Lupos source, implementation rationale, or parity report
was used.

## Result: FINDINGS

### R1 — the anonymous C storage members have been replaced by a distinct Rust API and a different access/lifetime boundary

`vendor/linux/include/uapi/linux/socket.h:16-27` makes the anonymous union's
anonymous struct members `ss_family` and `__data` direct members of
`struct __kernel_sockaddr_storage`; `__align` is the other direct union member.
The candidate instead adds the non-upstream `__storage` field at
`src/include/uapi/linux/socket.rs:31-33` and requires a caller to traverse
`__storage.__data` to reach `ss_family` or `__data` (and, in Rust, to perform a
union read).  This is not merely a spelling change: it removes the C header's
anonymous-member interface and introduces a new public wrapper/type boundary
where C has none.  The selected UAPI struct is embedded by value in external
UAPI records (for example `vendor/linux/include/uapi/linux/tcp.h:389-395`), so
the source must establish an exact representation/access mapping rather than
silently publish a distinct API.

Affected semantic records: `SC1-f1ee3c03ac7016bad779d05268f5dafde1dfa51675fa26a3c3c1ee393e055f62`,
`SC1-4da6075b61c111c195230c15c91e4bab2cf6870cd13654da8426982f25c9e7fd`,
`SC1-53bafcf1792c6c20c2b043f05172c19356b9d011b11fcc846fb4d7277d0d81f0`,
`SC1-b7a5fef3e117830359a1bda3aca9d645ec511a0002f5e4e4017c1089857f0e77`,
`SC1-2e046c2328620124ad1b6df28ded475be146c21d9d540aef627aaf940eeb9796`, and
`SC1-258297f519241c478efc0d4c7bc7faafa301609152430f0b18c7bb1f9c3ab476`.

### R2 — `char[126]` has been translated as `u8[126]` without proving the frozen C `char` value domain

The anonymous struct in `socket.h:18-22` declares `char __data[...]`, while
the candidate uses `[u8; ...]` at `socket.rs:19`.  Both have byte-sized
elements, but `char` has the frozen target compiler's C plain-char value domain
and promotion behavior; `u8` unconditionally changes it to 0..255.  This is a
public UAPI storage member, not an opaque Rust byte slice, and the frozen ABI
records leave the struct/anonymous-struct layout and alignment as semantic
closures.  No source evidence provided here proves that substituting unsigned
Rust bytes preserves every direct member consumer's signed values and C
promotion behavior on both approved architectures.

Affected semantic records: `SC1-f1ee3c03ac7016bad779d05268f5dafde1dfa51675fa26a3c3c1ee393e055f62`,
`SC1-53bafcf1792c6c20c2b043f05172c19356b9d011b11fcc846fb4d7277d0d81f0`,
`SC1-f77b0b3103c9fab1f49164221a414dd550843aa2886e34079bbe46d377c37476`, and
`SC1-a2b2b7a2a06e76b34041488189189370e5f11dc44a6cef816d5c9439e4786d2c`.

### R3 — untyped C integer macros were frozen as `usize`/`u32`, changing their expression contracts

`_K_SS_MAXSIZE`, both lock constants and the TX-rehash constants in
`socket.h:8,29-36` are untyped C integer-constant expressions.  They therefore
participate in C's `int` typing and integer promotions at their use sites.  The
candidate publishes `_K_SS_MAXSIZE` as `usize` and every remaining macro as
`u32` (`socket.rs:8,35-41`) without an exact, source-backed mapping for the
macro expression types.  This is observable in the pinned source: the
TX-rehash sentinel is intentionally compared through `(u8)val` in
`vendor/linux/net/core/sock.c:1276-1284`, while the lock mask participates in
bitwise operations on `sk_userlocks` at `sock.c:1651-1658,2147-2149`.
Changing the constants' public Rust types affects promotions, comparisons,
casts, and consumer type inference even when their numeric bits are the same.
No unsafe code is present to compensate for the changed scalar contracts.

Affected semantic records: `SC1-a75c107bfd1636105e138651186a6c745149d08957fdd28d99b8e82b95b1805c`,
`SC1-bee63a7504fee5537216523458e3cac1775867c9ef697b03e840a18dde64db56`,
`SC1-7b5667a0783d622a50736331a8b34e25f3d05070a5d97a8f2354fdd114cfe64f`,
`SC1-e40d6d53ad3ad92d8753753cf319c67bd95b94f47cb77fc9252144e2f2f128c6`,
`SC1-c81662f6cc0493a1eccbe4f56bc8bc40cdf1d89c557cd5934d004053098b121e`,
`SC1-5c6b0b1c8140919369ccf646330544df0edcbe120daca21da96aef157ec93b62`, and
`SC1-f3e0d7f438581bd246052ab48b749681c79f3f5e95133ed2b8dd4020f8535560`.

The candidate has no `unsafe` blocks, allocation, drop, borrow, or
concurrency mechanism to review.  `#[repr(C)]` does preserve the intended
outer union-style placement only if the element and macro-interface findings
above are resolved; it does not restore anonymous C member access or scalar
expression semantics.
