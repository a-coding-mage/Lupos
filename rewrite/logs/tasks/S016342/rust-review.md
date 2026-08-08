# Rust source review — S016342 (slot 2)

Status: **REJECT — source changes required before application.**

Scope reviewed manually: `vendor/linux/include/uapi/linux/psample.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the complete candidate
`src/include/uapi/linux/psample.rs`, and direct pinned consumers
`net/psample/psample.c` plus Generic Netlink declarations.  No compiler,
formatter, test, rust-analyzer diagnostic, historical Lupos source, or other
task evidence was used.

## Findings

1. **HIGH — named C enum constants were changed into scoped, nominal Rust enum
   variants.**

   `PSAMPLE_CMD_*` and `PSAMPLE_TUNNEL_KEY_ATTR_*` are C enumerator constants:
   their identifiers are in the ordinary C identifier namespace and, for these
   values, are integer constant expressions.  The candidate instead exposes
   only `psample_command::PSAMPLE_CMD_*` and
   `psample_tunnel_key_attr::PSAMPLE_TUNNEL_KEY_ATTR_*`, with nominal Rust enum
   types.  Thus it does not provide the upstream module-level symbols and does
   not preserve their integer-expression behavior.  This is operative in the
   pinned consumer: `net/psample/psample.c:119` performs
   `PSAMPLE_CMD_GET_GROUP + 1`; the same constants initialize `u8 cmd` fields
   and flow into `genlmsg_put(..., u8 cmd)` (`include/net/genetlink.h:191-195`,
   `:336-337`), while tunnel constants are passed to `int attrtype` Netlink
   helpers (`include/net/netlink.h:1403`, `:1455`, `:1539`, `:1665`).  Rust
   enum variants do not retain those operations or implicit conversions.

   Replace the named-enum variants with module-level integer constants matching
   the anonymous-enum mapping, or provide an explicitly evidenced compatible
   integer representation and all required module-level constant names.  Do
   not require downstream casts merely to reproduce C enumerator expressions.

2. **HIGH — the three C string-literal macros lost their C storage/terminator
   contract.**

   The candidate maps `PSAMPLE_NL_MCGRP_CONFIG_NAME`,
   `PSAMPLE_NL_MCGRP_SAMPLE_NAME`, and `PSAMPLE_GENL_NAME` to `&str`.  A Rust
   `&str` is a UTF-8 slice (pointer plus length) and contains no terminating
   NUL.  Upstream macro expansion supplies C string literals to fixed C-char
   array initializers: `struct genl_multicast_group::name[GENL_NAMSIZ]` at
   `include/net/genetlink.h:25-32` and `struct genl_family::name[GENL_NAMSIZ]`
   at `include/net/genetlink.h:55-75`; `net/psample/psample.c:33-35` and
   `:111` use the macros exactly there.  The C initialization copies the NUL
   terminator and zero-fills the rest of each 16-byte field.  `&str` cannot
   represent that ABI or initialize a C char array without a separate,
   non-equivalent conversion.

   Represent the macros with NUL-terminated byte storage suitable for copying
   into the exact fixed arrays, and make the consuming Rust layout preserve the
   C array initialization semantics.  Do not pass a Rust slice/string across a
   C-compatible boundary.

3. **MEDIUM — `#[repr(i32)]` makes an unresolved enum ABI and validity choice.**

   `ABI.tsv` still records both named enum layouts as `PENDING_REVIEW` for both
   architectures.  The header gives no explicit underlying C enum type, while
   the candidate fixes it to signed 32-bit Rust enums and thereby also imposes
   Rust's closed-discriminant validity model.  No FFI conversion boundary or
   validity proof exists in this file.  The source uses the named commands as
   integer protocol values, not as a Rust closed set; the downstream `u8` and
   `int` uses above demonstrate why this choice is material.

   Resolve the C enum representation from the pinned configuration/toolchain
   evidence or avoid introducing a nominal Rust enum where the translated
   interface needs integer constants.  Record the resulting ABI decision
   before this task can be closed.

## Checks with no additional finding

- All enumerator numeric sequences and `PSAMPLE_ATTR_MAX` evaluate to the
  upstream values (0 through 17 and 16 respectively); the latter remains a
  compile-time macro expression, so its nearby runtime-option comment does
  not make it mutable storage.
- `PSAMPLE_GENL_VERSION` is correctly value-mapped to 1.
- The candidate contains no `unsafe`, raw pointers, allocation, callbacks,
  interior mutability, `Drop`, synchronization, packed layout, or test code;
  those categories introduce no independent finding in this header.
- Attribute comments specify payload widths and network byte order, but this
  header declares only attribute IDs.  The candidate has no payload-layout
  abstraction to audit; consumers must continue to preserve those UAPI payload
  contracts.

## Pending-record disposition

The original `PENDING_REVIEW` ABI records for `enum psample_command` and
`enum psample_tunnel_key_attr` cannot be closed by this candidate.  The
anonymous enum declares no reusable C enum type; its integer constants are
value-correct.  There is no ownership, borrowing, pointer provenance, aliasing,
pinning, Send/Sync, refcount, RCU, callback, or Drop contract in the header
itself.  Named-enum layout/validity remains unresolved as described in finding
3.
