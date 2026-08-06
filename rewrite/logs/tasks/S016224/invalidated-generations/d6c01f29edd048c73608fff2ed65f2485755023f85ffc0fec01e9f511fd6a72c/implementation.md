# Implementation — S016224

Translated `include/uapi/linux/limits.h` to
`src/include/uapi/linux/limits.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Before editing, the required branch, P02 lease for S016224 (attempt 1), frozen
Linux pin, Phase 0 identity, scope/map/symbol manifests, and immutable queue
fingerprint `af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`
were verified. The task is a common, dependency-free, low-risk header selected
by both frozen x86_64 and AArch64 configurations.

The complete oracle header has only its include guard and thirteen unconditional
object-like UAPI macros: `NR_OPEN`, `NGROUPS_MAX`, `ARG_MAX`, `LINK_MAX`,
`MAX_CANON`, `MAX_INPUT`, `NAME_MAX`, `PATH_MAX`, `PIPE_BUF`,
`XATTR_NAME_MAX`, `XATTR_SIZE_MAX`, `XATTR_LIST_MAX`, and `RTSIG_MAX`.
Every replacement token is an unsuffixed decimal literal whose C type is `int`
on both frozen 64-bit targets. Each value fits `i32`; exported Rust constants
therefore preserve the C literal's signed 32-bit value in the translation. The
C macros themselves have no standalone storage, linkage, layout, or calling
ABI; their contextual C conversions occur in consumers. No configuration or
architecture conditional changes any declaration.

Inspected direct hierarchy/context: the kernel wrapper
`include/linux/limits.h`, UAPI consumers `fs.h`, `auto_fs.h`, and netfilter
headers, and selected uses in fixed-array bounds and UAPI aliases. `fs.h`
undefines only `NR_OPEN` in its own later preprocessor context; it does not
change this header's declaration. The header-closure record selects this header
for 8,864 AArch64 and 2,896 x86_64 consumers.

No copyright notice exists in the oracle beyond its retained SPDX identifier.
No historical Lupos source, compiler, formatter, test, runtime command,
rust-analyzer diagnostic, shared index, or non-leased source file was used or
changed.
