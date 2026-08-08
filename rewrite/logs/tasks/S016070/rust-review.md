# Rust semantic review — S016070 / P02 / attempt 1

Verdict: **FINDINGS**. This review used only the pinned header, its direct
pinned UAPI context, the candidate, the candidate snapshot, and the frozen
semantic proposal. No compiler, formatter, test, analyzer, or historical
Lupos source was used.

## RUST-1 — the extractor macros lost their contextual C integer contract

The five source definitions are untyped C function-like macros:
`BPF_CLASS`, `BPF_SIZE`, `BPF_MODE`, `BPF_OP`, and `BPF_SRC`
(`vendor/linux/include/uapi/linux/bpf_common.h:6,17,22,31,49`).  Each expands
to a masked expression after the C operand's ordinary integer promotions and
usual arithmetic conversions.  The candidate instead exposes five functions
of the fixed signature `fn(u32) -> u32` and makes every related literal a
`u32`.

That is not an equivalent source/ABI contract.  In the directly pinned UAPI
consumer layout, `struct sock_filter.code` is `__u16`
(`vendor/linux/include/uapi/linux/filter.h:24-28`); the C macros accept that
field directly and promote it.  The candidate rejects that operand without a
caller-invented cast.  Such a cast also changes the available operand domain
and the result/composition types for signed, narrower, and wider operands.
The fixed `u32` constants likewise cease to participate as the source `int`
literals do in contextual bitwise expressions and initializers.  No exact,
frozen mapping for these changed promotion, truncation, and composition rules
is provided.

Affected semantic records:

- `SC1-6af4819ddef62ae54fd69a0edb4073104c1854f29adf2a9ceb260e84287892d2`, `SC1-f1498133e218b5aa466df7cf7b7a2b05f17e1bacfd8060a71cc22f41ddd3f8c1`, `SC1-039327752eeb61aa1418b57b5eaa1c1d519e3895671def303462e71bd3d0f80b`, `SC1-0dc38d9fcb1fcd6f58abd317aa35147662e5692abdde38b4b5e156d627854733`, `SC1-74b642a8032ce43adff7ad61ad8efae62c75b01a1ceefeeb922fa334c81e4ed4` (aarch64 extractor selection expressions);
- `SC1-5d4a503466c79b8bfcd07232a23c9c5475c5f7573aa52503d5e7c6e92fc9df2f`, `SC1-213f1f21914debf8dccab71b438dac5f0e78153f71fa363b203ff285efe457af`, `SC1-e4552e868b51d92a7fe943c4efbc9cfe76af1a387e6379960bffd68881b0b100`, `SC1-eca34c112edcc1e87a561f29c4155d80d915b581dbc6adff279aa0051c14b0dd`, `SC1-8223029253e5cdc5551b06cbcd3ad1b7a5bc094e9ec55a3f70e6218a85ba21e1` (x86_64 extractor selection expressions).

## RUST-2 — `BPF_MAXINSNS` override behavior was replaced by an unconditional item

The pinned header deliberately guards its definition with `#ifndef
BPF_MAXINSNS` (`bpf_common.h:53-55`).  Thus a previously supplied value is
retained.  The candidate declares an unconditional `pub const BPF_MAXINSNS:
u32 = 4096`; it neither represents the conditional definition nor establishes
the required precedence when a surrounding translated UAPI context supplies
the symbol.  The candidate snapshot's assertion that the guard was preserved
has no corresponding mechanism in the source.

Affected semantic records:

- `SC1-7d8396bdec9228a1050a6ebb3bec97b5606c61d38b8ddc8118c92217343b7b66`, `SC1-cbf4f0196099088a88f8b4c38e1faaa0c6e2a31e917acb9f0bad02572ba3d4b3` (aarch64 `#ifndef BPF_MAXINSNS` and definition);
- `SC1-d6339a00b0012e26c20db82c7eadb934cccdcba7dd8911e6f9ec5020000bb6f4`, `SC1-e1f8ae753d50ee75f17cb09748f87a3e0194c2420668714991f6198fd3d58b03` (x86_64 `#ifndef BPF_MAXINSNS` and definition).

No Rust references, raw pointers, unsafe blocks, FFI declarations, repr-C
layouts, allocation paths, callbacks, interior mutability, or Drop behavior
occur in this candidate; those categories introduce no additional finding.
