# Parity review — S014598, attempt 3

Result: **FINDINGS**. This was a source-only review; no compiler, formatter, linker, test, or runtime tool was invoked.

Review identity: `P01`, attempt `3`, `REVIEWING`; pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`; Phase-0 identity digest `6e2df070e502b65ad41d9eeb061a402cf7b0c9c158bdc3428006babfd2917381`; frozen queue fingerprint `943e5f2626a4c95a4f0d2e83171907bf6a5b5b86611106cd497ee846f13da0c0`.

## Findings

### P001 — sealed semantic closure is not bound to the candidate being reviewed

The sealed proposal correctly hashes to `439bcfba6ecfc403726304a3ce0b6735a2f79fca7c40fd68f5e48b36e214c820`, but every one of its 11,617 records identifies candidate digest `35cdc7e8196d9ac2bd382ca3a91a0b2a8e9266caea8f158ba48f8902616e66ce`. The current candidate [`src/include/linux/pci_ids.rs`](/home/fenhir/Projects/lupos/src/include/linux/pci_ids.rs:1) hashes to `198a6f12bbb053e7f70ca55b3af8ffaba758159d4ebbfaf81041a6312834e24b`. Therefore the proposal's `COMPLETE` final states cannot attest to the exact candidate now under review.

Local source evidence: the source-wide scope semantic-status record, `SC1-63e16b9d32b57fa9035a58a16758551c034e2f995590e3b8f84fef0fbfccd4f9`, declares `COMPLETE` for the header bounded by Linux lines 10–3270, but carries the stale candidate digest. The same stale digest occurs in every proposal record. Regenerate and seal the closure against the present candidate, then independently review that new sealed proposal.

Affected closure record key: `SC1-63e16b9d32b57fa9035a58a16758551c034e2f995590e3b8f84fef0fbfccd4f9` (the root semantic-status assertion; its record set is stale as described above).

### P002 — header-guard macro is changed into a public integer constant

Linux symbol: `_LINUX_PCI_IDS_H`. The pinned header tests macro definedness at [`include/linux/pci_ids.h:10`](/home/fenhir/Projects/lupos/vendor/linux/include/linux/pci_ids.h:10), defines `_LINUX_PCI_IDS_H` with an empty replacement list at [line 11](/home/fenhir/Projects/lupos/vendor/linux/include/linux/pci_ids.h:11), and closes that preprocessor conditional at [line 3270](/home/fenhir/Projects/lupos/vendor/linux/include/linux/pci_ids.h:3270). The candidate instead adds an addressable public `core::ffi::c_int` value of `1` at [`src/include/linux/pci_ids.rs:12`](/home/fenhir/Projects/lupos/src/include/linux/pci_ids.rs:12). That is neither the empty replacement list nor a preservation of the source's definedness-based include guard, and it introduces a typed public item for a mechanism that has no C linkage or numeric replacement value.

The closure nevertheless marks the guard condition and macro complete for both selected architectures. A valid translation must preserve the one-time module/import mechanism without exporting a spurious integer value, and the closure must record that exact mechanism.

Affected closure record keys: `SC1-2e16f191c3a8d02e4dd25f5323173a400157fc7cb539f2a08ced3ca1eb9c42db`, `SC1-1f998305dd4438e313152f589d575009569fe52080cab8447cb3770d5f8eb4bd`, `SC1-8fe5caf89d71ced9219128329ea229853bc263ddb2d5a0d5e425667b5c5474c0`, `SC1-0fdf4131048c9d0958448dde7296f06bc7ad669340244d5d89519df29d80d1e9`, `SC1-af8d9b3887a5b15a289da00052bba2aba8cadb1936c923c7a83bd1bfe01414ee`, `SC1-245c6d13a18cb884d536cfd59b747f34570ef76d5307fc09d31a9d6eff9c583d`, `SC1-c9bbd44067140d83674a52fea7b537e89d9d4adb3ed615989b7cb531ba83ec67`, `SC1-05bed97add58c3f728628cd5fb2a272faac0dce438a738c9c04f80fa6a93eacc`.

## Exhaustive source checks

- The pinned header has exactly 2,903 `#define` directives: the empty header-guard macro plus 2,902 numeric object-like macros. Its only condition is the header guard; no architecture or Kconfig conditional occurs in the file.
- The candidate has exactly 2,903 `pub const` declarations and no other operative Rust item after provenance. All 2,902 numeric names and literal tokens match the corresponding pinned definitions. Every numeric token is an unsuffixed C `int`-representable hexadecimal literal; the largest is `PCI_CLASS_WIRELESS_WHCI` at Linux line 135, `0xd1010`, below `INT_MAX`. The candidate consistently uses `core::ffi::c_int`, which is the relevant signed C `int` type on both frozen targets. No numeric-macro value, name, configuration selection, ABI/linkage symbol, or branding discrepancy was found.
- The current proposal seal's stored digest matches its proposal bytes. After decoding TSV quoting, all 11,617 proposal records have complete source citations and final values that match the pinned header, both frozen configurations, and `rewrite/metadata/header_closure.tsv`; this does not cure P001's stale candidate binding or P002's false completion claim.
