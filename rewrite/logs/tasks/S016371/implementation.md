# S016371 implementation

- Task: `S016371`
- Linux source: `vendor/linux/include/uapi/linux/seg6_genl.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/uapi/linux/seg6_genl.rs`
- Architectures: `common` (the frozen x86_64 and AArch64 union)

The complete pinned header was read. It contains the SPDX notice, an include
guard, two string/integer macros, two anonymous integer enums, and two max
macros. The Rust module boundary supplies the one-definition behavior of the
include guard; no conditional configuration branch remains inside the header.

The C ordinary-namespace enum constants are represented as public `i32`
constants, matching the C enum integer values used by the Netlink policy and
generic-Netlink command declarations. The max constants retain the source
integer expressions (`__SEG6_ATTR_MAX - 1` and `__SEG6_CMD_MAX - 1`) rather
than duplicating their results. `SEG6_GENL_NAME` retains the source string
contents and `SEG6_GENL_VERSION` retains the hexadecimal integer literal.

Mapped symbols:

- `SEG6_GENL_NAME`, `SEG6_GENL_VERSION`
- `SEG6_ATTR_UNSPEC`, `SEG6_ATTR_DST`, `SEG6_ATTR_DSTLEN`,
  `SEG6_ATTR_HMACKEYID`, `SEG6_ATTR_SECRET`, `SEG6_ATTR_SECRETLEN`,
  `SEG6_ATTR_ALGID`, `SEG6_ATTR_HMACINFO`, `__SEG6_ATTR_MAX`, `SEG6_ATTR_MAX`
- `SEG6_CMD_UNSPEC`, `SEG6_CMD_SETHMAC`, `SEG6_CMD_DUMPHMAC`,
  `SEG6_CMD_SET_TUNSRC`, `SEG6_CMD_GET_TUNSRC`, `__SEG6_CMD_MAX`,
  `SEG6_CMD_MAX`

No functions, data layouts, locks, lifetimes, or unsafe operations are
present in this UAPI header. No compiler, formatter, linker, test, or runtime
command was run.
