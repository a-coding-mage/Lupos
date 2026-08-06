# Implementation: S014648

Translated `include/linux/pinctrl/pinctrl-state.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to the path-preserving destination
`src/include/linux/pinctrl/pinctrl-state.rs`.

The selected header has four operative macros and no declarations, types, or
conditional selected behavior beyond its include guard. Each macro is a C
string literal, represented by private static NUL-terminated `c_char` storage
and a public pointer constant with the original macro name. This preserves the
literal's static lifetime and array-to-pointer decay at its selected uses.

No compilation, formatting, tests, or other validation commands were run.
