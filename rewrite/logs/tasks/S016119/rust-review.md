# Rust source review — S016119 / attempt 1

Result: **FINDINGS.**

Review preconditions observed by direct read:

- branch reference: `refs/heads/feat/bun-like-rewrite-test`;
- queue row: `S016119`, `P01`, attempt `1`, status `REVIEWING`;
- pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`;
- current Phase-0 identity SHA-256: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`.

The initial seal-mismatch conclusion is withdrawn. Per
`tools/semantic_closure.py:524-562`, the proposal's `candidate_sha256` is the
hash of the fixed evidence path `candidate.diff`, not the destination Rust
source, and `queue_fingerprint` is the text value from the fingerprint file,
not a SHA-256 of `TRANSLATION_TASKS.tsv`.  Direct comparison with those two
unrelated hashes was invalid.

## Finding RUST-S016119-01 — missing selected UAPI string macro

The Rust candidate ends at line 769 with `ETHTOOL_MSG_KERNEL_MAX`. It contains
`ETHTOOL_GENL_NAME` and `ETHTOOL_GENL_VERSION`, but it exports no
`ETHTOOL_MCGRP_MONITOR_NAME`. The pinned header defines that selected UAPI
macro as the string literal `"monitor"`. Omitting it changes the public Rust
UAPI surface: a consumer cannot obtain the ethtool monitor multicast-group name
from this translation.

- Upstream evidence: `vendor/linux/include/uapi/linux/ethtool_netlink_generated.h:962`.
- Candidate evidence: `src/include/uapi/linux/ethtool_netlink_generated.rs:746-769`
  is the complete tail of the file; no constant follows the final kernel-message
  maximum.
- Closure keys: `SC1-f8cf4fbbeadbc3ab4bcc72ad23c9ca3c254ab45710284b27a72d5e65cf08d19c`
  (aarch64 selection expression) and
  `SC1-51610a311637fe94cd21d08cb4f8c6010d4ac871c4ac3df94782f09b58d54efa`
  (x86_64 selection expression), both for source line 962.
- Required resolution: add an exact Rust representation of the macro without
  changing its bytes or NUL/FFI use semantics, then regenerate the candidate
  evidence and repeat the reviews required by the pipeline.

Manual source review otherwise found this file contains only compile-time
constants and four integer aliases; it has no structs, unions, FFI functions,
unsafe blocks, ownership/pinning/aliasing operations, allocation, callbacks,
Drop behavior, tests, panic paths, or conditional Rust branches. The listed
enum chains preserve the upstream explicit bases, sequential increments, count
sentinels, and `MAX = CNT - 1` form in the inspected source. This does not
mitigate the omitted macro.

No compiler, formatter, test, rust-analyzer diagnostic, or other build action
was invoked.
