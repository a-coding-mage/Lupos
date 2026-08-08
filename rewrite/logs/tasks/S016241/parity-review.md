# Parity review — S016241, attempt 1, slot 1

Reviewer: parity reviewer / P01 / slot 1 / gpt-5.6-terra, high reasoning effort.

Result: **APPROVE — no findings.**

This was a manual, source-only comparison of the pinned Linux UAPI header
`vendor/linux/include/uapi/linux/membarrier.h` (revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`) with
`src/include/uapi/linux/membarrier.rs`. No compiler, formatter, linker, test,
runtime command, or diagnostics were used.

## Candidate and closure binding

- Candidate snapshot `candidate.diff` SHA-256:
  `fab668e88976ede3b404e700a2adc1209992b2f258e1828724279ca4dc56aca3`.
- Implementation evidence SHA-256:
  `024931491a2ff0416ab7238b916d8b8de07834f47aa159914ade0f76b249553a`.
- Sealed semantic-closure proposal SHA-256:
  `39d3d3f156de79b106f4470053ea7f5cf99feaa031f7331eb97f69cc6764edf2`;
  it contains 101 current-task records, all with final dispositions
  `COMPLETE` or `SOURCE_REVIEWED_VALUE`.
- The proposal binds the frozen Phase-0 identity
  `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
  and queue fingerprint
  `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`.

## Exhaustive parity check

The selected common header has only its C inclusion guard (Linux lines 1–2
and 169), `enum membarrier_cmd` (lines 148–163), and
`enum membarrier_cmd_flag` (lines 165–167). The Rust module boundary replaces
the textual C inclusion-guard mechanism; there are no configuration-dependent
or architecture-dependent branches in this file.

`membarrier_cmd` and `membarrier_cmd_flag` are represented as signed `i32`
aliases, preserving the `int`-valued command/flag interface. The candidate
defines every selected UAPI name exactly once. `MEMBARRIER_CMD_QUERY` remains
zero; `MEMBARRIER_CMD_GLOBAL` through
`MEMBARRIER_CMD_GET_REGISTRATIONS` retain their respective `1 << 0` through
`1 << 9` expressions; and `MEMBARRIER_CMD_FLAG_CPU` retains `1 << 0`.

Crucially, `MEMBARRIER_CMD_SHARED` is not a new bit: candidate line 25 aliases
`MEMBARRIER_CMD_GLOBAL`, exactly matching the Linux backward-compatible alias
at line 162. No command, flag, mask, name, value, active branch, or UAPI
category is omitted or added.

No semantic-proposal key is associated with a finding because this review has
no findings.
