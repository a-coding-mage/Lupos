# S016344 implementation

Translated the complete pinned `include/uapi/linux/psp.h` generic-netlink UAPI header at revision `425f94c2954b1fe80ebdbf9b29854e89750355df` to `src/include/uapi/linux/psp.rs`.

The header has one named C enum (`psp_version`), six anonymous numeric enum namespaces, and four object-like macros. `psp_version` and every anonymous enumerator use the frozen C `int` representation. All explicit and implicit enumerator values, including each private count sentinel and its public `MAX = sentinel - 1` value, are represented. The two family/multicast-group string macros are immutable NUL-terminated `c_char` arrays, preserving C string-literal storage and pointer decay; the family version remains a C integer constant.

The candidate contains no functions, structures, conditional configuration branches, driver code, Rust tests, or unsafe code. The source and YNL specification were read to confirm the complete protocol namespace and values. No build, formatter, compiler, test, or runtime command was run.
