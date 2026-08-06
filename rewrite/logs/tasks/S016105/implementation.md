# S016105 implementation

Translated `include/uapi/linux/dpll.h` to `src/include/uapi/linux/dpll.rs` from
the frozen Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The source contains fourteen named C enums and UAPI macros only. Each named
enum is represented by a `#[repr(transparent)]` newtype over `c_int`, preserving
the C enum ABI and a distinct tag identity while retaining all literal
enumerator values, private maxima, and public maxima. Literal integer macros
use `c_int`; string-literal macros retain their terminating NUL byte.

No configuration-dependent branch appears inside the header guard. No structs,
functions, allocation, ownership, locking, or unsafe operations are present.
