# S016228 Rust review

Reviewer role: Rust reviewer (independent source inspection only)

## Scope inspected

- Pinned oracle: `vendor/linux/include/uapi/linux/lockd_netlink.h`, lines 1-30,
  at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/uapi/linux/lockd_netlink.rs`, lines 1-30.
- Concrete generic-netlink use: `vendor/linux/fs/lockd/netlink.c`, lines 15-45;
  it uses the command and attribute constants in `u8`/index contexts and uses
  `LOCKD_FAMILY_NAME` to initialize the inline `char name[GENL_NAMSIZ]` member
  of `struct genl_family` (`include/net/genetlink.h`, lines 78-82).
- The selected symbol, ABI, and lifetime records for both architectures in
  `rewrite/SYMBOLS.tsv`, `rewrite/ABI.tsv`, and `rewrite/LIFETIMES.tsv`.

## Findings

### R1 — generated-source notice omitted (minor; must resolve)

The upstream UAPI header begins with four generated-source notices: it says
not to edit directly; identifies `Documentation/netlink/specs/lockd.yaml`; says
`YNL-GEN uapi header`; and identifies `tools/net/ynl/ynl-regen.sh` as the
regeneration command (oracle lines 2-5).  The candidate preserves SPDX but
replaces this with a new generic doc sentence, so it loses upstream generated
provenance.  Restore those notices as Rust comments, preserving the YAML path
and regeneration command.  This is a source/provenance correction only; it
must not add generated behavior or alter the immutable task provenance.

## Accepted mapping checks

- `LOCKD_FAMILY_NAME` is correctly represented by immutable static backing
  bytes `b"lockd\\0"`.  In C the macro expands to a static-duration string
  literal (`char[6]` under the selected `-funsigned-char` commands), which can
  initialize `genl_family.name` as an aggregate and decays only at pointer-use
  sites.  Keeping an `[u8; 6]` array rather than publishing a `*const c_char`
  preserves that aggregate source form; a consuming Rust translation must make
  any pointer conversion at its corresponding pointer-use site.
- The family version and both anonymous-enum enumerator sequences retain their
  exact values and the two `*_MAX = (__MAX - 1)` relationships.  These
  anonymous enum identifiers are C `int` integer constants in the shown value
  range, so `c_int` is correct on both approved Linux architectures.  The
  derived max constants remain `c_int` and therefore preserve the required
  explicit conversion point for narrower generic-netlink fields.
- The candidate introduces no unsafe code, layout-bearing aggregate, FFI
  declaration, pointer cast, ownership mechanism, or executable behavior.

No compiler, formatter, linker, test, runtime tool, rust-analyzer diagnostic,
or historical Lupos source was used.
