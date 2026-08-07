# Parity review — S016119 / attempt 1 / slot 1

Result: **FINDINGS**

Reviewed only the pinned `include/uapi/linux/ethtool_netlink_generated.h` at
Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate
`src/include/uapi/linux/ethtool_netlink_generated.rs`, and the current frozen
task/semantic records.  The queue row is `REVIEWING`, pipeline `P01`, attempt
`1`.  The sealed proposal binds Phase-0 identity
`0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2` and queue
fingerprint `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.

## PR-001 — missing UAPI multicast-group name macro

Linux symbol: `ETHTOOL_MCGRP_MONITOR_NAME`.

Pinned local evidence: `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h:962`
defines `ETHTOOL_MCGRP_MONITOR_NAME` as the string literal `"monitor"`.  The
candidate ends after `ETHTOOL_MSG_KERNEL_MAX` at line 769 and contains no
declaration for `ETHTOOL_MCGRP_MONITOR_NAME`; therefore this selected public
UAPI macro has no Rust-facing value/name mapping.

Semantic-closure evidence: the sealed proposal marks this operative macro
`COMPLETE` for both approved architectures, contrary to the candidate:

- `SC1-f8cf4fbbeadbc3ab4bcc72ad23c9ca3c254ab45710284b27a72d5e65cf08d19c`
  (`aarch64`, `selection_expression`)
- `SC1-192d1186924df08e8830f3f8542efe0db5ace55118ea2fde6c4bd6c732bac238`
  (`aarch64`, `status`)
- `SC1-51610a311637fe94cd21d08cb4f8c6010d4ac871c4ac3df94782f09b58d54efa`
  (`x86_64`, `selection_expression`)
- `SC1-b67b87eb5b6ab75f4f68f55af19306ea55ab6331d2662a352d098df1e8ea05d9`
  (`x86_64`, `status`)

Required resolution: add the source-equivalent public monitor-group name
constant and reseal/review the resulting candidate as required by the closure
workflow.

## Exhaustive inventory check

The candidate preserves the source order and names of all 649 enum constants;
each is represented as an `i32` constant, with source implicit successors
expressed as predecessor-plus-one and every source `*_MAX` expression retained
as count-minus-one.  The four named C enum tags are each represented by an
`i32` alias.  No conditional UAPI branch exists beyond the C include guard;
the Rust module itself supplies that inclusion mechanism.  No other
symbol/value/order/width discrepancy was identified by manual source review.

No compiler, formatter, linker, test, runtime tool, or compiler-backed
diagnostic was invoked.
