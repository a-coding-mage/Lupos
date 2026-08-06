# Parity review — S016284

Reviewer: parity reviewer (`gpt-5.6-terra`, high)

Reviewed the complete pinned `include/uapi/linux/netfilter/xt_LOG.h` against
`src/include/uapi/linux/netfilter/xt_LOG.rs`, the selected symbol inventory,
both frozen configurations, and the direct `xt_LOG.c`/`nf_log.h` consumers.
No compiler, formatter, linker, test, or runtime tool was invoked.

## Findings

No parity findings.

The candidate preserves all six `XT_LOG_*` flag values and the `XT_LOG_MASK`
value, including the deliberately excluded unsupported NFLOG bit.  The frozen
x86_64 and AArch64 Kbuild commands both specify `-funsigned-char`; therefore
the Rust `[u8; 30]` representation exactly preserves the source `char prefix[30]`
bytes.  `#[repr(C)]` preserves declaration order and gives `xt_log_info` the
same offsets (0, 1, 2), alignment (1), and total size (32) as the two
`unsigned char` fields followed by that array on both approved architectures.
The direct target consumer uses `sizeof(struct xt_log_info)` and reads exactly
these fields; no omitted conditionals, UAPI names, exported symbols, or
branding deltas were found.
