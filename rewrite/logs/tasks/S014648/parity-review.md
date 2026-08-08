# Parity review — S014648

Reviewer: parity_reviewer (`gpt-5.6-terra`, high)

Scope reviewed: `vendor/linux/include/linux/pinctrl/pinctrl-state.h` against
`src/include/linux/pinctrl/pinctrl-state.rs`, with direct pinned pinctrl
consumer/caller context. No compiler, formatter, test, or historical rewrite
source was used.

## Findings

1. **P1 — `PINCTRL_STATE_DEFAULT`, `PINCTRL_STATE_INIT`, `PINCTRL_STATE_IDLE`, and `PINCTRL_STATE_SLEEP` lose C string-literal representation.**
   Linux defines each macro as a C string literal (lines 33–36). That expression
   has a terminating NUL and decays to `const char *` when passed to
   `pinctrl_lookup_state`; see `pinctrl_bind_pins` in
   `drivers/base/pinctrl.c` and the `const char *name` parameter in
   `include/linux/pinctrl/consumer.h`. The candidate substitutes `&str`, which
   is a Rust slice (pointer plus length), is not NUL-terminated by its value,
   and cannot preserve the required `const char *` consumer contract. No
   source-proven Rust C-string/FFI mapping is present in this file.

2. **P1 — `PINCTRL_STATE_DEFAULT` no longer supports the macro’s adjacent-literal composition.**
   The C preprocessor expands `PINCTRL_STATE_DEFAULT " state not found for GPIO
   recovery\\n"` into one NUL-terminated literal; see
   `drivers/i2c/i2c-core-base.c:327`. A typed `&str` item cannot take part in
   C preprocessor literal concatenation, so the candidate changes the macro
   mechanism and its direct use in format-string arguments. The candidate has
   no source-level representation that preserves both the standalone
   `const char *` use and literal-concatenation behavior.

## Verdict

Reject. The source has no structs, functions, opaque types, or guard-driven
runtime branches beyond the include guard, but the four operative macros’ C
string and macro-expansion semantics are not preserved. Exact Rust mapping is
not established by the frozen record; this requires applier adjudication or a
block rather than acceptance.
