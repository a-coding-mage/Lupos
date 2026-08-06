# S016252 implementation record

- Task: `S016252`
- Source: `vendor/linux/include/uapi/linux/mptcp_pm.h`
- Destination: `src/include/uapi/linux/mptcp_pm.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (`x86_64`, `aarch64`)
- Implementer: Terra fallback, medium effort

Read the complete selected UAPI header and the task's symbol, ABI, lifetime,
scope, and file-map records. The header has no conditional configuration
branches other than its C include guard, no structures, and no functions.

The Rust file maps C named enum tags to `c_int` aliases, so all named
enumerator constants retain the C enum ABI. Every anonymous-enum enumerator
and every derived `*_MAX` macro is represented as a `c_int` constant with the
same source-order value and subtraction expression. `MPTCP_PM_NAME` preserves
the NUL-terminated C `char` string-literal value as a static array; the version
macro remains a C `int` constant. No behavior, storage layout, ownership, or
configuration-dependent logic is introduced by this declaration-only header.

No build, formatter, test, or runtime command was run.
