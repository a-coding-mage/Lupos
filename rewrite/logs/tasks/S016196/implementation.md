# S016196 implementation

- Task: `S016196`; pipeline: `P02`; lease owner: `codex-root-p02`.
- Source: `vendor/linux/include/uapi/linux/ioam6_genl.h`.
- Destination: `src/include/uapi/linux/ioam6_genl.rs`.
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df` (from `vendor/linux.SHA`).
- Architectures: `common` (selected by both `x86_64` and `aarch64`).

Read the complete pinned header, its wrapper `include/linux/ioam6_genl.h`, the
header closure metadata, the `include/net/ioam6.h` consumer, and the selected
call sites in `net/ipv6/ioam6.c` and `net/ipv6/exthdrs.c`. The frozen Kbuild
closure identifies the header through the built-in `net/ipv6/af_inet6.o` and
`net/ipv6/ioam6.o` objects for both architectures.

The destination preserves both generic-netlink names, version, event-group
name, all anonymous-enum constants and max calculations, and both tagged C
enums with C representation. C enum values are represented as their exact
zero-based `i32` discriminants; anonymous C enums remain global `i32` constants
so their names retain the C header's global use by netlink policy and message
callers. The schema length remains the source arithmetic `(255 * 4)`.

No historical Lupos source, compiler, formatter, linker, test, runtime, or
Git mutation was used.
