# S016386 applier resolution

The applier reopened the complete pinned
`vendor/linux/include/uapi/linux/socket.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate, both
replacement review reports, the frozen S016386 scope/symbol/ABI/lifetime rows,
and relevant pinned socket-header uses.  The source was adjudicated
independently of compiler feedback; no compiler, formatter, linker, test, or
diagnostic output was requested or used.

## Parity-review findings

### P1 — accepted and fixed: `__data` signedness

`socket.h:21` declares `char __data[...]`, and the pinned source unconditionally
adds `-funsigned-char` to `KBUILD_CFLAGS` at `vendor/linux/Makefile:607`.
Consequently the selected translation uses `[u8; __K_SS_DATA_LEN]`, not `i8`.
The private length constant is exactly `128 - size_of::<__kernel_sa_family_t>()`,
the Rust counterpart of the same source array bound.

### P2 — rejected as an ABI defect; representation evidence recorded

`socket.h:16-26` deliberately uses anonymous C aggregates so native C code can
write `storage.ss_family`, `storage.__data`, and `storage.__align` directly.
Rust has no anonymous `repr(C)` union/struct member promotion.  The named Rust
helpers retain the only cross-language contract carried by this type: the
outer storage has one offset-zero `repr(C)` union; its struct alternative has
`ss_family` at offset zero followed by the 126-byte payload, and its pointer
alternative supplies the source-required alignment.  This does not alter a C
consumer: the original pinned UAPI header remains the source interface for C,
and `include/linux/socket.h:63` aliases `sockaddr_storage` to this C struct.

The review correctly identified the language-level difference, so the source
comment was corrected not to claim anonymous member promotion.  A Rust caller
must explicitly select the named union alternative and uphold its active-member
invariant; no safe promoted-field accessor is invented, because that would
manufacture references over storage for which C permits arbitrary bytes.

### P3 — accepted and fixed: `_K_SS_MAXSIZE` expression type

`socket.h:8` defines an unsuffixed decimal literal, whose selected C expression
type is `int`.  `_K_SS_MAXSIZE` is now `i32`; only the private array-bound
constant converts its known non-negative value to `usize`.  The six remaining
socket lock/rehash macros remain `i32`, with `SOCK_BUF_LOCK_MASK` computed from
the two lock constants as in `socket.h:29-36`.

## Rust-review findings

### F01 — accepted and fixed: `_K_SS_MAXSIZE` public type

Resolved by the same `i32` public macro counterpart and private `usize` array
bound described for P3.  This preserves the C macro's normal integer expression
semantics while satisfying Rust's array-length requirement locally.

### F02 — accepted and fixed: plain-`char` payload type

Resolved by `[u8; __K_SS_DATA_LEN]`.  The pinned Linux source itself supplies
the required selected-configuration evidence: `vendor/linux/Makefile:607`
adds `-funsigned-char` to every `KBUILD_CFLAGS` invocation, and no conditional
in `socket.h:1-38` changes the payload declaration for either architecture.

### F03 — accepted and fixed: cross-context storage traits

`socket.h:17-25` makes `void *__align` an alternative union member used to
obtain implementation-required alignment; it does not express ownership of a
referent.  The header embeds the storage as address bytes in UAPI records, for
example `include/uapi/linux/in.h:216-239` and
`include/uapi/linux/tcp.h:390,417,434,468`.  Narrow documented `unsafe impl`
blocks therefore restore `Send` and `Sync` only for the outer storage type.
They do not dereference, own, or grant safe access to `__align`; a caller still
must meet the union active-member invariant.

## Closed task-local semantic conclusions

- `__kernel_sa_family_t` is the `unsigned short` field of `socket.h:10`, mapped
  to `u16` for both selected architectures.
- `__kernel_sockaddr_storage` is plain address storage: 128 bytes from the
  source bound, with the pointer union alternative preserving the selected
  64-bit x86_64/AArch64 alignment contract.  It has no allocation, drop,
  refcount, locking, RCU, callback, or ownership transition in this header.
- The anonymous C struct/union are layout alternatives, not independently
  linked symbols.  Rust names the unavoidable layout helpers but exposes no
  fabricated safe access path or ownership abstraction.
- The header guard carries no runtime semantic; all unconditional operative
  macros in `socket.h:8,29-36` are represented with their `i32` values and
  source expression relationship.

No finding remains unresolved at source-review level.
