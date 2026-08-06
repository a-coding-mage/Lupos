# S016371 implementation

Oracle: `vendor/linux/include/uapi/linux/seg6_genl.h` at frozen revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The common x86_64/AArch64 header is unconditional.  Its generic-netlink name
is retained as a five-byte `c_char` array including the C string literal's
terminating NUL.  Both anonymous enumerator groups and their derived maximum
macros are represented as `c_int` constant expressions, preserving the C enum
enumerator type and all ordinal values used by `net/ipv6/seg6.c`.

No ownership, allocation, locking, conditional configuration, or callable
behavior is present in this UAPI-only header.
