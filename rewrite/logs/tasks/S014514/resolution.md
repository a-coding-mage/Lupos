# Resolution — S014514

Reviewed the complete pinned `include/linux/nfs_iostat.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, both frozen target commands in
`rewrite/metadata/{x86_64,aarch64}/compile_commands.json`, and the selected
consumer at `fs/nfs/super.c:662`.  This resolution makes no build or test
claim.

## RUST-1 — resolved

`NFS_IOSTAT_VERS` is now backed by immutable static `[u8; 4]` storage equal to
`b"1.1\0"`.  The public macro-equivalent is a `*const core::ffi::c_char` formed
from that storage.  It is a one-word thin pointer, points at the first C
character, and the fourth byte is the required NUL.  This matches the C string
literal's selected decay at `seq_printf(..., "statvers=%s", NFS_IOSTAT_VERS)`;
it does not expose a Rust `&str` or a fat slice/reference at that boundary.

The frozen commands set `-funsigned-char` for both targets, hence byte storage
is appropriate and the cast is confined to the C-character-pointer boundary.
No dereference or unsafe Rust operation is introduced here.

## RUST-2 — resolved

The exact frozen commands for the header's selected NFS consumers are the
Clang 19.1.7 commands recorded at:

- `rewrite/metadata/x86_64/compile_commands.json` (`--target=x86_64-linux-gnu`)
- `rewrite/metadata/aarch64/compile_commands.json` (`--target=aarch64-linux-gnu`)

Neither command contains `-fshort-enums`; the frozen compiler is
`/usr/lib/llvm-19/bin/clang` version 19.1.7, as bound by
`rewrite/toolchain/TOOLCHAIN.tsv` and `rewrite/PHASE0_IDENTITY.tsv`.  The two
unforced C enums in the pinned header have only the values `0..8` and `0..27`.
Under these frozen Clang target commands their compatible integer type is C
`int`, with 32-bit representation on both selected LP64 targets.  The Rust
aliases now use `core::ffi::c_int`, rather than embedding an unexplained Rust
`i32`; this names the resolved C ABI type while preserving arbitrary integer
index values (unlike a Rust nominal enum).

All enumerator values and terminal bounds remain unchanged.  No source scope,
layout, linkage, ownership, locking, or configuration branch remains to
resolve for this header.
