# S016294 implementation

Translated the complete pinned UAPI header
`include/uapi/linux/netfilter/xt_state.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to its frozen path-preserving
destination `src/include/uapi/linux/netfilter/xt_state.rs`.

The header's three selected value-producing macros retain their C `int`
expression type as `core::ffi::c_int`. `XT_STATE_BIT` is a `const unsafe fn`
over the sibling translated `ip_conntrack_info` C-enum wrapper: its safety
precondition preserves the source macro's undefined-shift domain rather than
introducing a checked or panic-capable replacement. `XT_STATE_UNTRACKED`
continues to derive its bit position from `IP_CT_NUMBER`; it is not replaced
with a copied literal.

`struct xt_state_info` is a `#[repr(C)]` one-field struct with `statemask` as
`core::ffi::c_uint`, preserving the pinned source's `unsigned int` UAPI field
and its x86_64 four-byte C layout and alignment. This header has no packed
attribute, enum declaration, conditional branch beyond its include guard, or
function/linkage declaration. The C include guard is a preprocessing mechanism
and has no Rust ABI item.

No compiler, formatter, analyzer, linker, test, debugger, or runtime command
was run.
