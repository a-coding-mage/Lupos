# Resolution — S014648 attempt 1 (P02)

Applier: `applier` (`gpt-5.6-terra`, high)

I reopened the complete pinned
`vendor/linux/include/linux/pinctrl/pinctrl-state.h`, its direct declaration
context in `include/linux/pinctrl/consumer.h`, the direct core consumer in
`drivers/base/pinctrl.c`, and the adjacent-literal use in
`drivers/i2c/i2c-core-base.c:327`.  I also checked the frozen scope, symbol,
ABI, and lifetime records and the sealed candidate/proposal and both review
reports.  No compiler, formatter, test, analyzer, or historical rewrite source
was used.  The candidate was not edited.

## Finding dispositions

| Finding | Disposition |
| --- | --- |
| Parity P1 — all four `PINCTRL_STATE_*` values lose C string-literal/`const char *` representation | **Confirmed; unresolved.** Lines 33–36 of the pinned header define preprocessor string literals.  `pinctrl_lookup_state` takes `const char *name` in the pinned consumer header, and the pinned core passes these expansions directly.  A Rust `&'static str` is a pointer-and-length value and is not the C literal’s NUL-terminated pointer expression.  The frozen records contain no accepted static C-string storage, raw-pointer, or FFI ownership/provenance bridge for this header. |
| Parity P1 — `PINCTRL_STATE_DEFAULT` loses adjacent-literal composition | **Confirmed; unresolved.** The pinned `drivers/i2c/i2c-core-base.c:327` has `PINCTRL_STATE_DEFAULT " state not found for GPIO recovery\\n"`; C expands this into one literal before the call.  A Rust constant cannot be substituted into that token position, and no frozen Rust macro/module mechanism is established to preserve this form. |
| Rust P1-RUST-FFI-MACRO-LITERALS | **Confirmed; unresolved.** The same direct consumer contract requires a pointer to static NUL-terminated character storage.  Introducing a new byte-array/raw-pointer API here would require a cross-file FFI convention absent from the frozen task records, so it would be a new unreviewed design rather than a source-proven repair. |
| Rust P1-RUST-MACRO-EXPANSION | **Confirmed; unresolved.** `PINCTRL_STATE_*` and the selected `__LINUX_PINCTRL_PINCTRL_STATE_H` guard are C preprocessor names.  The sealed Rust file supplies neither a token-level substitution mechanism nor a source-proven visibility/include-guard equivalent.  The guard’s exact C macro visibility is selected in `SYMBOLS.tsv` and remains `PENDING_REVIEW`; the frozen records do not establish a Rust mapping that closes it. |

## Outcome

Manual source evidence establishes that the sealed `&str` substitution changes
the selected macro representation and a direct composition use.  It does not
establish an exact Rust-only representation of the C preprocessor tokens,
static NUL-terminated literal storage, and Linux-facing `const char *` use.
The task must remain unresolved rather than introducing a bridge outside the
frozen scope.  The pipeline is therefore blocked for the later source/ABI
workflow.
