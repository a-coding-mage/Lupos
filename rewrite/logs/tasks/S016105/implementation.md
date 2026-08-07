# S016105 implementation — attempt 2

Source: `vendor/linux/include/uapi/linux/dpll.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The fresh destination maps each C UAPI enum to a `u32` named type alias and
every enumerator to a same-named typed constant, preserving each explicit and
implicit ordinal and each public/private maximum relationship. Numeric macros
are `u32` constants. The two C string-literal macros are NUL-terminated byte
arrays, retaining their C literal object contents and extent. This header has
no conditional selected branches, functions, storage objects, or layouts.

No compiler, formatter, build, test, or diagnostic was run.
