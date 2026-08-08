# Resolution — S016342 (P02/a1)

## Decision

**BLOCKED.**  The candidate is not changed.  Its `candidate.diff` and both
reviews bind the existing candidate, and source-only evidence cannot establish
an exact replacement for the unresolved named C-enum ABI/value domain and the
C string-literal macro representation on both frozen architectures.

## Source evidence reopened

- `vendor/linux/include/uapi/linux/psample.h:35-60` declares the two named C
  enums without an explicit underlying type, and `:66-68` declares the three
  names as C string-literal macros.
- `vendor/linux/net/psample/psample.c:32-35`, `:40-48`, `:88-160`, and
  `:225-282` use the command and tunnel identifiers unqualified in ordinary C
  integer-expression contexts and initialize generic-netlink name arrays from
  the string macros.
- `vendor/linux/include/net/genetlink.h:25-32`, `:55-75`, and `:191-195`
  establish `char name[GENL_NAMSIZ]` storage and `u8 cmd` consumer fields;
  `vendor/linux/include/net/netlink.h:1389-1707` takes tunnel attribute types
  as `int`.
- Frozen `rewrite/ABI.tsv` rows for `enum psample_command` and `enum
  psample_tunnel_key_attr` are `PENDING_REVIEW` for x86_64 and aarch64.  The
  corresponding frozen lifetime records remain pending as well.  No admitted
  source evidence fixes an underlying enum ABI or validates a closed Rust enum
  value domain for both targets.

## Dispositions

1. **Parity P1 / Rust HIGH — `PSAMPLE_CMD_*` namespace and integer use:**
   confirmed.  The candidate's associated variants are not the upstream
   module-level C enumerator identifiers and cannot serve the pinned
   unqualified integer-expression consumers.  Not repaired: choosing a Rust
   integer representation or conversion boundary would require the unresolved
   named-enum ABI/value-domain decision.

2. **Parity P1 / Rust HIGH — `PSAMPLE_TUNNEL_KEY_ATTR_*` namespace and
   integer use:** confirmed.  The candidate's associated variants omit the
   unqualified C enumerator interface used as `int attrtype` arguments.  Not
   repaired for the same unresolved representation and consumer-boundary
   reason.

3. **Parity P1 / Rust HIGH — C string-literal macro storage and terminator:**
   confirmed.  `&str` does not reproduce the NUL-terminated byte array supplied
   by the C macros to fixed `char name[GENL_NAMSIZ]` initializers.  No frozen
   ABI record defines the Rust-facing fixed-array initializer/conversion
   boundary, so selecting byte-array exports or a conversion mechanism here
   would be an unreviewed design rather than a source-proven translation.

4. **Parity P1 — selected `__UAPI_PSAMPLE_H` guard macro absent:** confirmed.
   The frozen symbol record selects the guard macro, whereas the candidate has
   no corresponding item or source-proven equivalent.  Rust module loading is
   not evidence for preserving the selected visible preprocessor mechanism.

5. **Parity P1 / Rust MEDIUM — `#[repr(i32)]` named-enum ABI and closed-value
   choice:** confirmed.  The pinned header gives no explicit underlying type;
   the frozen ABI/lifetime records do not resolve it for either target.  The
   candidate therefore cannot be accepted as an exact two-architecture ABI or
   semantic replacement.

6. **Parity P1 — candidate snapshot incomplete and stale:** confirmed.  The
   419-byte `candidate.diff` contains only provenance and a comment, while the
   current candidate defines additional items; its architecture provenance also
   differs from the queue's `common` value.  Recreating it would require a
   replacement candidate followed by fresh independent review, which is not
   possible until the ABI and macro-representation blocker is resolved.

No compiler, formatter, linker, test, runtime command, analyzer diagnostic,
historical Lupos source, or external source was used.  No source or frozen
manifest was changed.
