# Rust review: S014648

Result: **reject — one correctness finding**.

## R1 — object-like string-literal macros were changed into fixed pointer values

`PINCTRL_STATE_DEFAULT`, `PINCTRL_STATE_INIT`, `PINCTRL_STATE_IDLE`, and
`PINCTRL_STATE_SLEEP` are object-like C preprocessor macros.  Each expands to
a string-literal token (`"default"`, `"init"`, `"idle"`, or `"sleep"`), whose
source-level type is a `char[N]` array and which decays to `char *` only in the
contexts where C applies array-to-pointer conversion.  The candidate instead
exposes four `pub const *const c_char` values pointing at one fixed private
`static` array each.

This is not a representation-preserving lowering of the public macro
semantics: a pointer value cannot participate in C adjacent-string-literal
concatenation, array initialization/indexing, or other literal/array contexts.
The pinned source uses the former directly in
`drivers/i2c/i2c-core-base.c:327`:

```c
dev_dbg(dev, PINCTRL_STATE_DEFAULT " state not found for GPIO recovery\\n");
```

After macro expansion that is one `"default state not found for GPIO
recovery\\n"` literal.  The candidate's `*const c_char` cannot express that
operation and also changes the expansion's array type and its C `char *`
decay into an immutable raw-pointer interface.

The applier must replace this pointer-constant model with the project’s
faithful macro/literal representation and establish the corresponding
downstream translation/FFI rule.  It must preserve the exact byte sequences,
trailing NUL, static lifetime, and literal-versus-pointer use distinctions;
private singleton storage plus public pointer constants is insufficient.

Evidence reviewed: pinned
`vendor/linux/include/linux/pinctrl/pinctrl-state.h:33-36`, candidate
`src/include/linux/pinctrl/pinctrl-state.rs:14-34`, and pinned selected
consumer `vendor/linux/drivers/i2c/i2c-core-base.c:327`.
