# S016189 parity review (slot 1)

Outcome: FINDINGS — reject candidate pending correction.

Scope checked: pinned `vendor/linux/include/uapi/linux/input-event-codes.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, fresh
`src/include/uapi/linux/input-event-codes.rs`, and the frozen S016189 queue/
scope/file-map/symbol context.  The queue row was `REVIEWING`, leased to P01,
with the expected source, destination, `common` architecture, and matching
provenance SHA.  The worktree HEAD reference was
`refs/heads/feat/bun-like-rewrite-test`.

Method: manual source inspection plus a lexical, comment-stripped text
comparison only; no compiler, formatter, test, or diagnostics were invoked.
The pinned header has 795 non-guard `#define` records.  The candidate has 795
same-named `pub const` starts in the same order.  Of those, 792 become
separate comment-stripped `i32` declarations with matching RHS token spelling;
the three cases below merge into the following declaration because the
translated multi-line C comment is not followed by a Rust statement
semicolon.

1. Linux symbols `KEY_SWITCHVIDEOMODE` and `KEY_KBDILLUMTOGGLE` are not valid
   separate Rust constants.  The pinned header defines `KEY_SWITCHVIDEOMODE`
   as `227` at lines 307-308 and independently defines
   `KEY_KBDILLUMTOGGLE` as `228` at line 309.  Candidate lines 313-315 end
   the copied block comment and immediately begin the next `pub const`, with
   no `;` after the `KEY_SWITCHVIDEOMODE` expression.  After comments are
   lexically removed the source is `... = 227 pub const
   KEY_KBDILLUMTOGGLE ...`, so neither declaration preserves its Linux
   macro/constant definition.

2. Linux symbols `KEY_BRIGHTNESS_AUTO` and `KEY_BRIGHTNESS_ZERO` are not
   valid separate Rust constants.  The pinned header defines the former as
   `244` at lines 330-332 and the latter as the alias
   `KEY_BRIGHTNESS_AUTO` at line 333.  Candidate lines 336-339 similarly
   omit the Rust `;` after the multi-line comment.  The comment-stripped
   token sequence merges `244` and the following `pub const`, so the value
   and alias are not emitted as the two independent Linux definitions.

3. Linux symbols `SW_RFKILL_ALL` and `SW_RADIO` are not valid separate Rust
   constants.  The pinned header defines `SW_RFKILL_ALL` as `0x03` at lines
   938-939 and `SW_RADIO` as its alias at line 940.  Candidate lines 944-946
   omit the terminating `;` after the multi-line comment.  The resulting
   comment-stripped sequence merges `0x03` with `pub const SW_RADIO`, losing
   both independently usable definitions and the alias relationship.

4. The candidate does not preserve this UAPI header's compile-time
   preprocessor interface.  The pinned file's only conditional is the
   `_UAPI_INPUT_EVENT_CODES_H` include guard at lines 16-17 and 1016; its
   own required-use notice at lines 6-7 states it is included by C and
   devicetree source and therefore contains comments and defines only.  Its
   795 definitions are `#define` macro names visible to those preprocessors.
   Candidate lines 1-1021 contain Rust documentation/comments and `pub const`
   items, with no include guard, preprocessor definitions, or bridge that
   preserves repeated-include and C/devicetree macro visibility.  Thus even
   apart from findings 1-3, a consumer requiring the pinned UAPI header's
   conditional/macro mechanism cannot receive this candidate's definitions.

Additional source-review observations: all 795 source macro names appear in
the candidate in source order; the 792 structurally separate candidate values
use `i32`, which matches the pinned header's ordinary `int`-range literal and
alias expressions on the approved architectures.  No source conditional other
than the include guard exists.  I found no candidate `todo!`,
`unimplemented!`, panic placeholder, Rust test configuration, or Lupos
branding; the Linux source and SHA provenance and upstream UAPI SPDX notice
are present.  These observations do not mitigate the findings above.
