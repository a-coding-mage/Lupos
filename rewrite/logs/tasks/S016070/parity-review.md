# Parity review — S016070 / P02 attempt 1

Status: FINDINGS

## P1 — function-like UAPI macros were narrowed into `u32` functions

Linux defines `BPF_CLASS`, `BPF_SIZE`, `BPF_MODE`, `BPF_OP`, and `BPF_SRC` as
untyped C macros (`include/uapi/linux/bpf_common.h:6,17,22,31,49`).  Their
operand and result participate in C's usual arithmetic conversions: a
`__u16` `sock_filter.code` consumer is promoted to `int` (see
`include/uapi/linux/filter.h:18-22`), while an unsigned-long or other wider
consumer keeps that width/sign domain.  The candidate instead accepts only
`u32` and returns `u32`.  This changes accepted operand types, required casts,
result type/signedness, and contextual use of every macro.  The numerical
masks and one evaluation do not establish parity.

Affected closure keys:

- `SC1-6af4819ddef62ae54fd69a0edb4073104c1854f29adf2a9ceb260e84287892d2`, `SC1-f1498133e218b5aa466df7cf7b7a2b05f17e1bacfd8060a71cc22f41ddd3f8c1`, `SC1-039327752eeb61aa1418b57b5eaa1c1d519e3895671def303462e71bd3d0f80b`, `SC1-0dc38d9fcb1fcd6f58abd317aa35147662e5692abdde38b4b5e156d627854733`, `SC1-74b642a8032ce43adff7ad61ad8efae62c75b01a1ceefeeb922fa334c81e4ed4` (aarch64)
- `SC1-5d4a503466c79b8bfcd07232a23c9c5475c5f7573aa52503d5e7c6e92fc9df2f`, `SC1-213f1f21914debf8dccab71b438dac5f0e78153f71fa363b203ff285efe457af`, `SC1-e4552e868b51d92a7fe943c4efbc9cfe76af1a387e6379960bffd68881b0b100`, `SC1-eca34c112edcc1e87a561f29c4155d80d915b581dbc6adff279aa0051c14b0dd`, `SC1-8223029253e5cdc5551b06cbcd3ad1b7a5bc094e9ec55a3f70e6218a85ba21e1` (x86_64)

## P2 — `BPF_MAXINSNS`'s caller-controlled preprocessor guard is absent

Linux only defines `BPF_MAXINSNS` when the includer has not already defined it
(`include/uapi/linux/bpf_common.h:53-55`).  The candidate always publishes
`pub const BPF_MAXINSNS: u32 = 4096`; it cannot preserve a prior UAPI macro
definition or the conditional branch.  Its own candidate summary incorrectly
calls that behavior preserved.  No source evidence establishes a Rust mapping
that retains this public preprocessor contract.

Affected closure keys:

- `SC1-7d8396bdec9228a1050a6ebb3bec97b5606c61d38b8ddc8118c92217343b7b66`, `SC1-cbf4f0196099088a88f8b4c38e1faaa0c6e2a31e917acb9f0bad02572ba3d4b3` (aarch64)
- `SC1-d6339a00b0012e26c20db82c7eadb934cccdcba7dd8911e6f9ec5020000bb6f4`, `SC1-e1f8ae753d50ee75f17cb09748f87a3e0194c2420668714991f6198fd3d58b03` (x86_64)

## P3 — object-like C macros lost their `int` literal type and macro context

Every opcode/field value in the source is an unsuffixed hexadecimal integer
literal macro (for example `BPF_LD` at line 7 and `BPF_JSET` at line 48).  On
both approved targets these are `int` expressions, usable in preprocessor and
C expression contexts.  The candidate silently changes all of them to `u32`
items.  Numeric equality is insufficient for the UAPI's source-level type and
macro contract.  This report maps the representative class field records;
the same defect applies to the remaining object-like macros in their source
order.

Affected closure keys:

- `SC1-36b16138a4a790744c6301200088c405620cf6de9db7ed8bbd8a434dde47053a`, `SC1-6f5446abcae1e3a045d1eb5d5bbb03cdc87b69eb75857fe38f27bb9f3ba793d0` (aarch64: `BPF_LD`, `BPF_JSET`)
- `SC1-3baaad02c8e49002be49d94114929af76ae0d0ba10fc8923e8763f95bf08a0c2`, `SC1-8ee86e0a180e6478a06a83a8da5994c6c20882b588be8a950da9beff1dae0975` (x86_64: `BPF_LD`, `BPF_JSET`)

No compiler, formatter, test, analyzer, or runtime command was used.
