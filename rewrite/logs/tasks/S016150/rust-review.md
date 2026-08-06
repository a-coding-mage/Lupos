# S016150 Rust review (slot 2)

Reviewed `src/include/uapi/linux/hsr_netlink.rs` against pinned
`vendor/linux/include/uapi/linux/hsr_netlink.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the aarch64 frozen configuration,
and the recorded HSR header consumers.

## Result

No Rust-specific defect found. No source change is requested.

## Checks

1. **Anonymous enum and integer semantics — accepted.** The two C declarations
   at upstream lines 21--34 and 39--48 have no enum tag, no declared object,
   and no externally usable enum type. Under the pinned `-std=gnu11` command,
   their enumerator identifiers are `int` constant expressions. Candidate
   lines 21--32 and 37--44 preserve every identifier, ordering, and value as a
   signed `c_int`; on the frozen `aarch64-linux-gnu` target this is the C
   32-bit `int`. It correctly does not introduce a `repr(C)` enum, an ABI
   object, or an invented enum-tag type.

2. **Derived macro expressions and conversion boundaries — accepted.** C
   `HSR_A_MAX` and `HSR_C_MAX` are parenthesized subtraction expressions whose
   operands/results are signed `int`. Candidate lines 33 and 45 retain them
   as `c_int` expressions, yielding 10 and 6 respectively, without an
   unsigned-literal or narrowing substitution. The recorded operating
   consumer uses attributes in APIs taking `int` (`nla_put*`) and also lets C
   perform its ordinary context conversions for Generic Netlink `u8` command
   fields and `unsigned int` `maxattr` (see
   `include/net/genetlink.h:78-85,191-198,336-337` and
   `include/net/netlink.h:562-570,1403-1407,1455-1459`). Future Rust consumer
   translations must make those target-type conversions explicitly; this
   provider correctly preserves the original expression type rather than
   silently changing the public constants to `u8`, `u32`, or `usize`.

3. **Selected UAPI/provenance surface — accepted.** The include guard is the
   only conditional in the C header and creates no runtime or Kconfig branch.
   The frozen aarch64 configuration has `CONFIG_HSR=m`; header-closure metadata
   records three selected Rust-translation consumers under `net/hsr/hsr.o`.
   Candidate lines 1--5 exactly retain SPDX, source path, pinned revision,
   architecture, and task provenance. It adds no unauthorized branding or
   configuration-dependent interface.

4. **Rust safety and failure behavior — accepted.** The file has no FFI item,
   object layout, `unsafe`, pointer/reference creation, mutable global,
   allocation, `Drop`, panic/unwrap/expect path, test configuration, or
   placeholder. Its only import is `core::ffi::c_int`, appropriate to the
   frozen C integer type.

No finding remains for applier action from this Rust review.
