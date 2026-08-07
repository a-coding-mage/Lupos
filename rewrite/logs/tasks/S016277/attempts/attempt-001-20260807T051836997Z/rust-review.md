# Rust source review — S016277

Reviewed `src/include/uapi/linux/netfilter/nf_tables.rs` against the pinned
`vendor/linux/include/uapi/linux/netfilter/nf_tables.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus the sealed S016277 semantic
closure proposal.  This was a manual source review only; no compiler,
formatter, test, or runtime tool was invoked.

## Findings

### S016277-RUST-001 — missing selected macro and unresolved Rust name

Linux defines `NFT_NAME_MAXLEN` as `256` at `nf_tables.h:5`.  The candidate
does not define that public constant, but uses it in
`NFT_TABLE_MAXNAMELEN`, `NFT_CHAIN_MAXNAMELEN`, `NFT_SET_MAXNAMELEN`, and
`NFT_OBJ_MAXNAMELEN` at Rust lines 966–969.  This omits a selected UAPI macro
and leaves those four definitions without a Rust value source.  Restore
`NFT_NAME_MAXLEN` with the correct value and retain the aliases.

Affected sealed keys:
`SC1-66873d0a196b05dff7c9dab16657c689011c9cd59348c2ca05e2cfa5e6f39151`,
`SC1-7ddce31af563ed905e9513264656031d098dd1548f916be9d6af5f7ff3fb0f3c`,
`SC1-a862c18781aa48075ae5221d0f0709f7b9444bacd41de653ff27ee7f285f1b79`,
`SC1-582576f0ac8666c01a5be5ae626815f504cdff84c3ee55e8804e17624305c855`.

### S016277-RUST-002 — `__KERNEL__` condition was replaced by an unproven Cargo feature

Linux exposes `NFT_REG32_MAX` only inside the C preprocessor `#ifdef
__KERNEL__` at `nf_tables.h:49–51`.  The candidate replaces that mechanism
with `#[cfg(feature = "__KERNEL__")]` at Rust line 973.  No supplied frozen
source evidence establishes that a Cargo feature with this literal name is
defined and enabled for both selected architecture builds.  The sealed
proposal marks the condition and macro complete for both x86_64 and aarch64.
Thus the Rust condition can omit a selected kernel-visible UAPI constant and
does not preserve the frozen C macro-visibility mechanism.  The applier must
establish and document an exact configuration mapping, or emit the selected
constant according to the frozen kernel translation configuration.

Affected sealed keys:
`SC1-d39378de21edce766b4d3fa9f0b36598225068f6c254404b7d69f2aacfbfd7c8`,
`SC1-8082b91497bf74ee245e9d90bde52477457e907cdfa08d46ac5998ad3905df79`,
`SC1-08e33fd3d749293fdf31ae3b01814315e063f1f08d542c4e905be14b22302a77`,
`SC1-56e19584fe02938c2f1058ca04fee0f14dbb50fb05106692edd7d0de48c7f28d`,
`SC1-6d77279184de85dfcc9d0a65d678cc9f2b0d423313ab7263d7bcb65576ec98ab`,
`SC1-2b04ba9aa0ff75680681b0128d985bda79a1f1e9b6cf5b5ed73c788ca64f1875`,
`SC1-91c69283317f93b278bca6df507aa8894ab78c89ae7a02ae146d3bae0c663824`,
`SC1-69eeb13964c5549ca8fe8e3311a4dde4456a8a0b8f76139a097864834ee812de`.

### S016277-RUST-003 — unsigned C macro lost its required Rust type

Linux defines `NFT_DATA_RESERVED_MASK` as `0xffffff00U` at
`nf_tables.h:509`; the `U` makes the UAPI expression unsigned.  The candidate
uses an untyped `0xffffff00` at Rust line 991.  That literal has no expected
type and exceeds signed `i32`; it cannot faithfully represent the C unsigned
macro in this form.  Declare it explicitly as `u32` with the same value, so
it agrees with the nearby `nft_data_types`/`NFT_DATA_VERDICT` unsigned value.

Affected sealed keys:
`SC1-51db7868968f49beb96d1bfa36ec6e0eda3819d022f1f673b41ed61681ad2521`,
`SC1-126674503e7226df3dbe9a9e97ff568780599b9c1efaede45a5a3e3dd0adbe8e`,
`SC1-77336f1e70f11411d900d12129a17a98f9502aed1fef15994b89650a1f1f450e`,
`SC1-f64368167405977a52b8fe084c4a3c1004be6c1b5ebf6c2d8c04b2ccba344745`.

## Additional review coverage

The candidate contains no `unsafe`, FFI declarations, pointers, references,
interior-mutability types, callbacks, allocation, `Drop`, indexing, panics,
or Rust test configuration.  Its 115 `pub type` aliases correspond in count
to the 115 source enums; no representation-bearing structs, unions, packing,
or calling conventions are introduced.  Apart from the findings above, the
enumerator chains and small masks were manually checked as explicit `i32`
constants, while `nft_data_types` and `NFT_DATA_VERDICT` correctly use `u32`.
The report is a rejection pending resolution of all three findings.
