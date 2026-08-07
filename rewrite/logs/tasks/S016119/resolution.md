# Application resolution — S016119 / P01 / attempt 1

## Preconditions and authority

- The task is leased to `P01` in `APPLYING`, attempt `1`.
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Phase-0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`.
- The source authority is `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h`.

## Finding dispositions

### PR-001 — RESOLVED

`vendor/linux/include/uapi/linux/ethtool_netlink_generated.h:962` defines
`ETHTOOL_MCGRP_MONITOR_NAME` as the C string literal `"monitor"`.  The final
candidate now exports, in the matching source-tail position,
`pub const ETHTOOL_MCGRP_MONITOR_NAME: &[u8; 8] = b"monitor\\0";` at
`src/include/uapi/linux/ethtool_netlink_generated.rs:770`.  This represents
the literal's seven payload bytes and implicit C NUL with the same public,
fixed-array UAPI convention already used for `ETHTOOL_GENL_NAME` at line 16.

The associated sealed-record keys are
`SC1-f8cf4fbbeadbc3ab4bcc72ad23c9ca3c254ab45710284b27a72d5e65cf08d19c`,
`SC1-192d1186924df08e8830f3f8542efe0db5ace55118ea2fde6c4bd6c732bac238`,
`SC1-51610a311637fe94cd21d08cb4f8c6010d4ac871c4ac3df94782f09b58d54efa`, and
`SC1-b67b87eb5b6ab75f4f68f55af19306ea55ab6331d2662a352d098df1e8ea05d9`.
Their sealed semantic values already correctly record the exact macro and
`COMPLETE` status for both architectures, so their semantic disposition is
`RESOLVED_NO_CHANGE`; the source candidate, not the frozen semantic decision,
was corrected.

### RUST-S016119-01 — RESOLVED

The same upstream definition and fixed-byte C-string representation resolve the
Rust UAPI surface omission without a pointer, ownership, allocation, layout,
or unsafe change.  The associated keys are
`SC1-f8cf4fbbeadbc3ab4bcc72ad23c9ca3c254ab45710284b27a72d5e65cf08d19c` and
`SC1-51610a311637fe94cd21d08cb4f8c6010d4ac871c4ac3df94782f09b58d54efa`;
their semantic disposition is likewise `RESOLVED_NO_CHANGE`.

## Adjacent inventory recheck

The complete pinned header has only three non-guard `#define` macros:
`ETHTOOL_GENL_NAME`, `ETHTOOL_GENL_VERSION`, and
`ETHTOOL_MCGRP_MONITOR_NAME` (lines 10, 11, and 962).  The candidate now has
public mappings for all three.  The include guard has no Rust symbol analogue;
the Rust module supplies inclusion semantics.  The reviewed enum inventory and
its source order remain unchanged.

## Semantic closure

`semantic-closure-final.tsv` was prepared from the sealed 3,977-record
proposal.  The two disposition rows cover every finding and exactly the record
keys attested by their independent reviews.  No frozen Phase-0 manifest was
changed.

No compiler, formatter, linker, test, runtime tool, or compiler-backed
diagnostic was invoked.
