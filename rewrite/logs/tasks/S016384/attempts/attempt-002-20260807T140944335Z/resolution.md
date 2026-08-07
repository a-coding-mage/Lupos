# S016384 applier resolution — attempt 2

Task: `include/uapi/linux/snmp.h` -> `src/include/uapi/linux/snmp.rs`  
Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`  
Pipeline: `P02`

## Finding dispositions

| Finding | Disposition | Pinned-source evidence and resolution |
| --- | --- | --- |
| P1 — eight selected UAPI enum names omitted | Accepted and source-corrected | The eight anonymous enums in `vendor/linux/include/uapi/linux/snmp.h` terminate at lines 61, 101, 119, 147, 167, 309, 348, and 372 with `__IPSTATS_MIB_MAX = 38`, `__ICMP_MIB_MAX = 30`, `__ICMP6_MIB_MAX = 7`, `__TCP_MIB_MAX = 16`, `__UDP_MIB_MAX = 10`, `__LINUX_MIB_MAX = 136`, `__LINUX_MIB_XFRMMAX = 33`, and `__LINUX_MIB_TLSMAX = 18`, respectively.  Each now appears as the corresponding `pub const NAME: i32 = VALUE;` immediately after the preceding enum member in the Rust translation. |
| RUST-S016384-01 — eight terminal enum constants omitted | Accepted and source-corrected | The same eight source enumerators are public named C `int` values.  The added Rust module constants retain their source names and represented `i32` values; all eight values fit in `i32`. |

## Source-only inventory after correction

The pinned header has eight anonymous enum declarations, 296 enumerators (including the eight terminal names), and the two value macros `__ICMPMSG_MIB_MAX` and `__ICMP6MSG_MIB_MAX`.  The corrected Rust module has 298 public `i32` constants: all 296 enum values plus those two macros.  The eight enum-declaration semantic records and the two macro semantic records remain within the task's selected scope; no declaration record is omitted by this source correction.

No compiler, formatter, linker, test, diagnostic, or historical source was used.

## Binding and required disposition

The reviewed attempt was bound to `candidate.diff` SHA-256 `b28480343761524695d038fe26af475c81671e60238a2003fac9b47df5cf91d1` and `implementation.md` SHA-256 `1805f1270306be72f5047f68a43d99b0da9cf34618989a914f3971f388555de7`.  The corrected current evidence hashes are `a2b27cdd1055739f432d380a7a338876d3c4edfd2f7d3d5cb4f7accaa6f3f2bf` for `candidate.diff` and `53166298b026f42a52bd2508128ce91784df4a23c4c0452c2e4c9dd566cc8a71` for `implementation.md`; the corrected destination SHA-256 is `e86c9fe316b9f0e903c7af524c18c12455ebe4826d2d3781f5618bb698aa964d`.  Therefore the sealed proposal and both review attestations for attempt 2 no longer bind to the corrected candidate.

This attempt MUST NOT transition to `DONE` and must not be resealed while it is `APPLYING`.  A coordinator-controlled requeue to a fresh attempt, with a new candidate snapshot, implementation evidence, proposal/seal, and independent reviews, is required.  No queue transition was made by this applier.
