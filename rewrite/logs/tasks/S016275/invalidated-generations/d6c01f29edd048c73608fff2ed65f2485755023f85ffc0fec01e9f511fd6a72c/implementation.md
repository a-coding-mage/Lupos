# S016275 implementation

Translated `include/uapi/linux/netfilter/nf_log.h` to its one-to-one destination
`src/include/uapi/linux/netfilter/nf_log.rs`.

The complete source has eight object-like integer macros and no type, storage,
function, conditional configuration branch, or include dependency.  Each macro
is represented as a public `core::ffi::c_int` constant, preserving the C type
of its unsuffixed hexadecimal or decimal integer literal on both frozen
64-bit Linux architectures.  The C include guard has no Rust counterpart.

Evidence consulted: pinned header lines 1-15; S016275 scope and symbol rows;
both frozen configurations; and frozen netfilter callers that use the values
as masks and the prefix length as an array bound.  No compiler, formatter,
linker, test, debugger, or analyzer was run.
