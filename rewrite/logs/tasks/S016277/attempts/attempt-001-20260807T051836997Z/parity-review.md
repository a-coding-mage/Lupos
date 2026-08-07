# Parity review — S016277 / P02 / slot 1

Result: FINDINGS.  This is a manual source review only; no compiler, formatter,
linker, test, or diagnostics were invoked.

Reviewed candidate: `src/include/uapi/linux/netfilter/nf_tables.rs`
(`sha256=69d35b1ea696c7aa8163566e4f74a4559c90ee3f0f3649db04d7480262123bec`).
Pinned source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.  The task was `REVIEWING`,
attempt 1, pipeline P02 when reviewed.  Frozen bindings inspected: Phase-0
identity `6e2df070e502b65ad41d9eeb061a402cf7b0c9c158bdc3428006babfd2917381`,
queue fingerprint `943e5f2626a4c95a4f0d2e83171907bf6a5b5b86611106cd497ee846f13da0c0`,
and the sealed semantic proposal
`15cc08a4b54cbf264b911e86e97cf570377aa06ad2ef72fa4cfb9bb475fdf65e`.
The sealed proposal is bound to the current candidate snapshot hash
`17a22303bc334c702d2aab48ed176b2a47dc39ab3e26a7103bcd36e70e920f88`
and implementation-evidence hash
`7b1761f5b35a44bf2903a1404c40d57aad3fa10e80bd2166183facd256757bf4`.

## Findings

1. **F001 — Linux symbols `NFT_NAME_MAXLEN` and `_LINUX_NF_TABLES_H` are
   omitted.**  The pinned header defines the include guard at lines 2–3 and
   `NFT_NAME_MAXLEN` as `256` at line 5.  The candidate contains neither
   name.  Its lines 966–969 nevertheless define four public aliases in terms
   of the absent `NFT_NAME_MAXLEN`; manually reading the Rust items establishes
   that there is no in-module declaration to which those expressions resolve.
   `SYMBOLS.tsv` inventories both names as selected operative macros for both
   architectures, but the candidate does not provide their mapped public
   definitions.  This is a missing UAPI macro surface and invalidates the
   proposal's `COMPLETE` decisions for those records.

2. **F002 — Linux symbol `NFT_META_OIFTYPE` is missing and shifts the trailing
   `enum nft_meta_keys` values.**  In the pinned header, lines 977–980 place
   `NFT_META_OIFTYPE` immediately after `NFT_META_IFTYPE` (with
   `NFT_META_IIFTYPE` a macro alias), so its mechanical value is 9 and
   `NFT_META_SKUID` follows at 10.  Candidate lines 430–433 go directly from
   `NFT_META_IFTYPE` to `NFT_META_SKUID`; no `NFT_META_OIFTYPE` definition
   exists anywhere in the candidate.  Consequently the candidate assigns
   `NFT_META_SKUID` 9 and shifts every following enumerator through
   `NFT_META_BRI_IIFHWADDR` by one.  Pinned callers use this symbol in
   `net/netfilter/nft_meta.c` (lines 259, 398, and 532), confirming that this
   is operative netfilter behavior, not documentation.  The frozen inventory
   lists the symbol for both architectures at source line 979.

3. **F003 — Linux symbol `NFT_REG32_MAX` has an unproven, different selection
   mechanism for the `__KERNEL__` branch.**  The pinned header's lines 49–51
   expose the macro exactly when the C preprocessor macro `__KERNEL__` is
   defined.  Candidate lines 973–974 substitute
   `#[cfg(feature = "__KERNEL__")]`, a Rust package-feature predicate.  No
   mapping from the selected C preprocessor condition to that Rust feature is
   present in the candidate or the frozen task records; the frozen configs are
   not evidence of such a mapping.  The distinction matters: pinned kernel
   callers such as `net/netfilter/nft_immediate.c:28` and
   `net/netfilter/nft_lookup.c:128–129` require `NFT_REG32_MAX`.  Source-only
   evidence therefore cannot establish that the candidate exposes it in the
   same contexts; this selected conditional must remain unresolved rather than
   be marked complete.

4. **F004 — Linux enum types lack source-supported ABI closure.**  The pinned
   file declares 115 named C enum types.  The candidate replaces them with
   primitive aliases (predominantly `i32`, but `nft_data_types` is `u32`).
   The latter source enum includes `NFT_DATA_VERDICT = 0xffffff00U` at pinned
   line 506, so its signedness/representation needs an explicit target-ABI
   decision rather than inference from the translated literal.  `ABI.tsv`
   still records layout and alignment as `PENDING_REVIEW` for these enum types
   on both architectures.  The sealed proposal marks all 5,737 records
   `COMPLETE` using only header/config citations and gives no source-derived
   resolution of enum compatible type, width, signedness, or FFI representation.
   The broad `COMPLETE` closure is unsupported; this finding is deliberately
   global because it affects the proposal's entire type-ABI field set.

The manual inventory found all remaining selected enum-constant names present
in the candidate; no structs, unions, functions, allocation, locking, RCU, or
branding deltas exist in this header.  The candidate has no Rust test,
placeholder, `unsafe`, or non-allowlisted branding text.  Those observations do
not mitigate F001–F004.

## Semantic-closure binding

Slot 1 records `FINDINGS` against the sealed proposal hash above.

- F001 keys: `SC1-23ac1417533d2a77fdf65c9dd66c1326864352e1c980b6e3de139e72e44ac1df`, `SC1-718d782ea7243e0f119fc82e5d6499d912463ad4689e816fa18f546d568fe382`, `SC1-76a8a01eb1091f8e925afa59b02b25c05963e55e37cebd09ade11e01a6059ac8`, `SC1-80d0f2193cfd148497bdca24e7d2682a56030ea0bdcd703230a36d5211a8c2e5`, `SC1-66873d0a196b05dff7c9dab16657c689011c9cd59348c2ca05e2cfa5e6f39151`, `SC1-7ddce31af563ed905e9513264656031d098dd1548f916be9d6af5f7ff3fb0f3c`, `SC1-a862c18781aa48075ae5221d0f0709f7b9444bacd41de653ff27ee7f285f1b79`, `SC1-582576f0ac8666c01a5be5ae626815f504cdff84c3ee55e8804e17624305c855`.
- F002 keys: `SC1-2783671a1f82d84fce2ad50954b69036323fbb636dc681a70737dbb77ea78389`, `SC1-25585f6ece861caffd010762effb3149681d112bb96a5bd0245cc2e444b522f5`, `SC1-d1d143b9d8324e7996858a14f6e4f57078430adb63130c6c656e145791d4d9fc`, `SC1-ae65646adca9edaa4831a03aa3ce81180adb605f98f36343861fe12f516bb624`.
- F003 keys: `SC1-d39378de21edce766b4d3fa9f0b36598225068f6c254404b7d69f2aacfbfd7c8`, `SC1-6d77279184de85dfcc9d0a65d678cc9f2b0d423313ab7263d7bcb65576ec98ab`.
- F004 is a global proposal finding; no finite record-key subset would
  accurately represent the unsupported blanket completion of its type-ABI
  closure.
