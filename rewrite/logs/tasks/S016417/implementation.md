# S016417 implementation

- Source: `vendor/linux/include/uapi/linux/thermal.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/thermal.rs`.
- Scope: all unconditional common thermal UAPI definitions for the frozen x86_64/aarch64 union.
- Six C enum tags use transparent `c_int` wrappers, retaining the C ABI and allowing every C integer representation; all enumerators preserve their source values, including sentinel values and derived `*_MAX` expressions.
- The three string-literal macros retain NUL-terminated static `c_char` arrays, matching C static storage and expression-context array decay through `.as_ptr()`.
- No conditional source definitions exist. No tests, drivers, module indexes, or build actions were added.
