# S016228 implementation record

- Task: `S016228`
- Linux source: `vendor/linux/include/uapi/linux/lockd_netlink.h`
- Destination: `src/include/uapi/linux/lockd_netlink.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (x86_64 and AArch64)
- Scope: header guard, family name/version macros, both anonymous enum
  namespaces and their explicitly derived MAX values.

The C anonymous enums publish integer constants in the ordinary C identifier
namespace; each Rust constant is therefore an `i32`, matching the C enum's
underlying ABI type for the pinned source and uses. Implicit enumerators are
spelled explicitly to preserve their values and namespace. `LOCKD_FAMILY_NAME`
retains the NUL terminator required when the macro initializes Linux's fixed
generic-Netlink family-name character array. No conditional branch other than
the source include guard is present in the pinned header; Rust module loading
provides the equivalent single-definition boundary.

Direct consumers inspected: `vendor/linux/fs/lockd/netlink.c`,
`vendor/linux/fs/lockd/netlink.h`, and the server attribute accesses and
Netlink output in `vendor/linux/fs/lockd/svc.c`. The constants preserve the
attribute indexes, command values, family version, and family-name bytes used
by those consumers.
