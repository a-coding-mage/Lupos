# Parity review — S016342 (slot 1, P02/a1)

## Scope and method

Reviewed `vendor/linux/include/uapi/linux/psample.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/psample.rs`, the task row, frozen symbol/ABI/lifetime
records, and direct pinned generic-netlink and psample consumers.  This was a
manual source review only; no compiler, formatter, test, or diagnostic output
was used.

## Result

**REJECT — source changes are required.** The anonymous attribute enumerators
have the correct numeric sequence, but the candidate does not preserve the
named-enumerator interface, C string-literal representation, or the selected
include-guard macro.  The candidate snapshot also cannot establish the reviewed
candidate boundary.

## Findings

1. **P1 — Linux symbols `PSAMPLE_CMD_SAMPLE`, `PSAMPLE_CMD_GET_GROUP`,
   `PSAMPLE_CMD_NEW_GROUP`, and `PSAMPLE_CMD_DEL_GROUP` are no longer global
   enumerator identifiers.**

   Local evidence: `include/uapi/linux/psample.h:35-40` declares `enum
   psample_command`; C places each of those enumerators in the including
   translation unit's ordinary identifier namespace.  The direct consumer
   `net/psample/psample.c:88`, `:103`, `:119`, `:154`, and `:160` uses these
   names unqualified, and `:40-48` passes the enum command to `genlmsg_put`.
   Candidate `src/include/uapi/linux/psample.rs:28-33` instead makes them
   associated variants of `psample_command`, with no exported global constants
   of the Linux names.  This changes both the selected symbol namespace and
   direct caller interface; it also requires a new qualification mechanism
   absent from Linux.

2. **P1 — Linux symbols `PSAMPLE_TUNNEL_KEY_ATTR_ID` through
   `__PSAMPLE_TUNNEL_KEY_ATTR_MAX` are likewise re-scoped and no longer retain
   their Linux global enumerator interface.**

   Local evidence: `include/uapi/linux/psample.h:42-60` declares the complete
   `enum psample_tunnel_key_attr` sequence as global C enumerators.  The direct
   consumer `net/psample/psample.c:225-282` passes the unqualified identifiers
   to `nla_put_*`, whose `attrtype` parameter is `int` in
   `include/net/netlink.h:1389`, `:1416`, `:1455`, `:1524`, `:1539`, `:1665`,
   `:1692`, and `:1707`.  Candidate `psample.rs:37-56` makes every identifier
   an associated Rust enum variant and supplies no global constants.  The
   source interface and the C implicit integer-use behavior are therefore
   missing.

3. **P1 — Linux macros `PSAMPLE_NL_MCGRP_CONFIG_NAME`,
   `PSAMPLE_NL_MCGRP_SAMPLE_NAME`, and `PSAMPLE_GENL_NAME` lost their C string
   literal / NUL-terminated-array semantics.**

   Local evidence: `include/uapi/linux/psample.h:66-68` defines each as a C
   string literal.  `net/psample/psample.c:32-35` initializes generic-netlink
   multicast-group names with the first two, and `:110-113` initializes the
   family name with `PSAMPLE_GENL_NAME`.  The receiving ABI stores these in
   `char name[GENL_NAMSIZ]` (`include/net/genetlink.h:29-32` and `:78-82`), and
   `net/netlink/genetlink.c:466-470` explicitly requires a terminating NUL.
   Candidate `psample.rs:59-61` exports Rust `&str`, a UTF-8 slice/fat
   reference with no trailing NUL and no C-character-array representation.
   It cannot preserve those initializer and layout semantics.

4. **P1 — Linux macro `__UAPI_PSAMPLE_H` is selected but absent.**

   Local evidence: the frozen `SYMBOLS.tsv` row for S016342 selects operative
   macro `__UAPI_PSAMPLE_H` at `include/uapi/linux/psample.h:3` for both
   architectures; the pinned header establishes it in the guard at `:2-3`.
   Candidate `psample.rs:1-62` defines no corresponding item or documented
   equivalent.  A Rust module may avoid repeated textual inclusion, but that
   is a changed mechanism and does not reproduce the selected macro visible to
   dependent source.

5. **P1 — `enum psample_command` and `enum psample_tunnel_key_attr` have an
   unresolved ABI and value-domain change.**

   Local evidence: the frozen ABI records for both enum types and both
   architectures are `PENDING_REVIEW`; the candidate unilaterally fixes each
   to `#[repr(i32)]` at `psample.rs:26` and `:35`.  The pinned source passes
   `enum psample_command` to the `u8 cmd` parameter of `genlmsg_put`
   (`net/psample/psample.c:40-48`; `net/netlink/genetlink.c:888-900`) and
   passes tunnel enumerators to `int attrtype` parameters as above.  C enum
   values remain integer-compatible and are implicitly converted at those
   calls; the candidate creates closed Rust enum types, requiring a new cast
   and rejecting an integer value outside the declared variants.  No frozen
   source evidence establishes that signed `i32` layout and this restricted
   Rust value domain are the exact ABI/semantic replacement on both approved
   architectures.  This uncertainty must be resolved from source evidence,
   not compiler output.

6. **P1 — the required candidate snapshot does not describe the reviewed
   candidate for Linux symbols `PSAMPLE_ATTR_IIFINDEX` and
   `PSAMPLE_GENL_NAME` (and therefore not the rest of this header).**

   Local evidence: `rewrite/logs/tasks/S016342/candidate.diff` ends at line 11
   after a comment and contains none of the definitions.  The reviewed source
   defines `PSAMPLE_ATTR_IIFINDEX` at `psample.rs:7` and `PSAMPLE_GENL_NAME`
   at `:61`; their Linux definitions are `psample.h:6` and `:68`, respectively.
   The supplied candidate diff therefore cannot be the required evidence
   snapshot for the 62-line reviewed source.  Recreate the candidate snapshot
   from the exact source that will be applied, then repeat independent review
   of any changed candidate.

## Checked mappings without a separate finding

- `PSAMPLE_ATTR_IIFINDEX` through `__PSAMPLE_ATTR_MAX` retain the pinned
  anonymous-enum values 0 through 17 in candidate lines 7-24.
- `PSAMPLE_ATTR_MAX` retains the pinned expression/result 16 at candidate
  line 58.
- `PSAMPLE_GENL_VERSION` retains value 1 at candidate line 62.
- The SPDX expression and pinned Linux revision match the source header and
  `vendor/linux.SHA`; no branding delta is allowlisted or introduced.

No queue, source, or manifest files were edited by this review.
