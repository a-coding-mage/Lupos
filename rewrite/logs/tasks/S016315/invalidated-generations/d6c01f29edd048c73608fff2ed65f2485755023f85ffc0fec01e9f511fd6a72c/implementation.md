# Implementation — S016315

Source: `vendor/linux/include/uapi/linux/nfsacl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected common UAPI header contains only an include guard and fifteen
object-like macros.  The guard has no runtime or ABI counterpart in a Rust
module.  Each macro is an unsuffixed C integer constant expression whose value
fits the frozen 32-bit C `int`; each is represented as a public `i32` constant.

The selected x86_64 and AArch64 configurations contain no conditional branch
in this header.  Consumer context in NFS client/server code uses these values
as RPC procedure numbers and ACL bit flags, so names and values are retained
exactly.  The corresponding internal header adds non-UAPI helpers and is out
of this task's mapped source scope.
