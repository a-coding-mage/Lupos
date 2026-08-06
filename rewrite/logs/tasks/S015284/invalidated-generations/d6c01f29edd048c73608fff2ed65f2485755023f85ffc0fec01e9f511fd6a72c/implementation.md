# S015284 implementation

- Task: `include/linux/uts.h` -> `src/include/linux/uts.rs`
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `x86_64,aarch64`

The complete source header contains only the include guard and three guarded
value macros. The Rust file maps `UTS_SYSNAME`, `UTS_NODENAME`, and
`UTS_DOMAINNAME` to public string constants with their frozen-configuration
values. Both frozen configurations set `CONFIG_DEFAULT_HOSTNAME="(none)"`, so
`UTS_NODENAME` and `UTS_DOMAINNAME` resolve to the same string. These values
initialize the corresponding `new_utsname` fields in `init/version-timestamp.c`;
runtime hostname and domain changes operate on those fields rather than these
initial values.

No types, storage, linkage, ownership, synchronization, error paths, or unsafe
operations are present in the source header. No compiler, formatter, test, or
runtime command was run.
