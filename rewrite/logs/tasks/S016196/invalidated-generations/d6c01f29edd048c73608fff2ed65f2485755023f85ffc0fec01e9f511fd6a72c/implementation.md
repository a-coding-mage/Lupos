# S016196 implementation

Translated `include/uapi/linux/ioam6_genl.h` to
`src/include/uapi/linux/ioam6_genl.rs` from the pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen common source
surface selected by both `x86_64` and `aarch64` configurations.

The two anonymous C enums are retained as explicitly typed sequential `c_int`
constants, including their private `__*_MAX` sentinels and public `*_MAX`
expressions. The named C enum tags retain distinct transparent `c_int` ABI
wrappers; their values accept all C `int` bit patterns rather than only the
currently named protocol values. `IOAM6_GENL_NAME` and
`IOAM6_GENL_EV_GRP_NAME` retain their C string-literal array representation as
NUL-terminated static `c_char` arrays, with `.as_ptr()` used at a C
pointer-decay boundary. `IOAM6_MAX_SCHEMA_DATA_LEN` remains the C `int`
constant expression `255 * 4`.

The source header defines no objects with ownership, lifetime, allocation,
locking, or executable behavior. No build, formatting, test, or runtime
command was run.

## Applier correction

The initial candidate's nominal enum wrappers and static string arrays were
superseded during final review. The final source uses `c_int` aliases for the
named C tags while retaining every enumerator as a `c_int` expression, and uses
array constants for the two literal macros so no Rust object replaces the
upstream macro/aggregate-initializer surface.
