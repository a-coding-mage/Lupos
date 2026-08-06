# Implementation — S016395

Translated `include/uapi/linux/sunrpc_netlink.h` to
`src/include/uapi/linux/sunrpc_netlink.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The candidate preserves the complete selected common UAPI surface for the
frozen architecture union: `sunrpc_cache_type` with its C `int` representation,
all seven anonymous-enum enumerator sets and their derived `*_MAX` values,
the family version, and all three string-literal macro values.  String macros
are represented by NUL-terminated static backing storage and public
macro-equivalent `*const c_char` values, matching C array decay.

There are no configuration branches beyond the C include guard, no ownership
or cleanup behavior, and no executable code in this header.  No build,
format, test, or runtime command was run.
