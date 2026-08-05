# S009463 implementation start

- Task: `S009463`
- Linux source: `net/wireless/nl80211.c`
- Destination: `src/net/wireless/nl80211.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common`
- Queue state: `IN_PROGRESS`
- Pipeline: `P01`
- Worker: `codex-p01`
- Attempt: `1`

## Mechanical preflight

The frozen scope maps this source to both architecture objects:

- `x86_64:net/wireless/nl80211.o`
- `aarch64:net/wireless/nl80211.o`

The pinned source is 23,459 lines and includes the complete generic-netlink
wireless configuration interface, policy tables, command dispatch, and
cfg80211/rdev integration. The Phase 0 symbol, ABI, and lifetime rows are
mechanical file records with semantic fields `PENDING_REVIEW`; they do not
constitute permission to infer ownership or synchronization behavior.

No destination source has been created yet. Translation proceeds only after
the complete source and the required pinned local headers, callers, callees,
Kconfig branches, and Kbuild evidence have been reviewed. No compiler, linker,
formatter, test, emulator, debugger, or runtime command was run.

## Direct source context

The source directly includes 15 pinned Linux headers and four local wireless
headers: `core.h`, `nl80211.h`, `reg.h`, and `rdev-ops.h`. Those local headers
total 2,613 lines and expose the cfg80211 registration, regulatory, operation
dispatch, and wireless-device contracts consumed by this file.

The source contains conditional branches for `CONFIG_NL80211_TESTMODE`,
`CONFIG_PM`, `CONFIG_CFG80211_WEXT`, `CONFIG_CFG80211_CRDA_SUPPORT`, and
`CONFIG_INET`. These branches are part of the common task's frozen union and
must be preserved as configuration behavior, not collapsed based on one
architecture's generated configuration.

The implementation surface includes policy tables, multicast-group tables,
netlink command handlers, dump callbacks, notification helpers, and the
`nl80211_init`/`nl80211_exit` registration lifecycle. Because the Phase 0
semantic rows remain `PENDING_REVIEW`, no ownership, locking, callback, RCU,
or ABI contract has been inferred in this preflight.

## Translation progress

## Blocked before implementation completion

This task cannot truthfully be marked `IMPLEMENTED`. The destination contains
only the small validation-helper subset (23 Rust functions) while the pinned
23,459-line source exposes approximately 140 C function definitions plus
policy tables, generic-netlink operation tables, multicast groups, dumps,
notifications, command handlers, cfg80211/rdev dispatch, and init/exit
lifecycle code. The frozen `SYMBOLS.tsv`, `LIFETIMES.tsv`, and `ABI.tsv` rows
for this task are each only a mechanical file record with `PENDING_REVIEW`;
they provide no selected-symbol mapping or ownership/locking/ABI records from
which the omitted interface and callback contracts can be established.

The task is therefore blocked, rather than represented as a complete
translation. The existing partial candidate is retained for the eventual
scope/ABI resolution but is not an implementation-complete artifact. The
provenance architecture field was corrected to the queue's required `common`.
No compiler, formatter, linker, test, emulator, debugger, or runtime command
was run.

The destination now contains the provenance header and the first translated
validator, `validate_supported_selectors` (Linux lines 301-313). The
translation preserves the C `u8` length conversion, payload offset, high-bit
test, `-EINVAL` result, and absence of an extack write on failure. The task is
still `IN_PROGRESS`; this file is not an `IMPLEMENTED` candidate because the
remaining source symbols, policy tables, handlers, and lifecycle code are not
yet present.

The adjacent `validate_nan_cluster_id` block (Linux lines 316-334) is now also
translated, including the six-byte length check, four-byte OUI prefix check,
diagnostic trace, `bad_attr` update, and `-EINVAL` paths. Its extack layout is
recorded as the pinned Linux `struct netlink_ext_ack` prefix and remains subject
to the independent Rust/ABI review required before implementation completion.

The `validate_nan_avail_blob` block (Linux lines 336-368) is now translated as
well. The candidate preserves the minimum-header check, attribute ID check,
unaligned little-endian length read, formatted extack messages, bounded extack
buffer, and exact `-EINVAL` exits. The source file remains incomplete and the
queue status remains `IN_PROGRESS`.

The following `validate_nan_ulw` block (Linux lines 370-425) is now translated,
including its three-byte header guard, required attribute ID, accepted lengths
16/18/21/23, remaining-buffer bound, little-endian decoding, and all formatted
extack diagnostics.

The HE inline helper needed by `validate_he_capa` is now translated from
`include/linux/ieee80211-he.h`, including the packed fixed-element layout, MCS
NSS sizing, PPE threshold bit count, cumulative bounds checks, and `u8` wrapping
arithmetic. `validate_he_capa` now delegates to that local helper and preserves
Linux's `-EINVAL` result.

The UHR operation and capability inline size checks needed by the two UHR
validators are now translated from `include/linux/ieee80211-uhr.h`. The
candidate preserves the fixed packed sizes, little-endian parameter fields,
optional DPS/NPCA/P-EDCA/DBE sections, DBE capability maps, `from_ap = false`,
and cumulative length guards.

The `validate_ie_attr` block (Linux lines 274-289) is now translated using the
pinned `struct element` iteration rules from `include/linux/ieee80211.h`. It
preserves the two-byte header requirement, element payload bound, completion
condition, extack message, and `-EINVAL` result for malformed trailing data.

The `validate_beacon_head` block (Linux lines 226-271) is now translated with
the pinned S1G frame-control predicates, optional-field lengths, management and
S1G packed offsets, Linux `ieee80211_hdrlen` call, and trailing element parser.
All malformed-head paths use the Linux diagnostic and `-EINVAL` result.
