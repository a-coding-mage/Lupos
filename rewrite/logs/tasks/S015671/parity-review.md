# Parity review — S015671 / P01 attempt 1

Reviewer: parity-reviewer (`gpt-5.6-terra`, high)

## Result

APPROVE — no source-level parity finding.

## Evidence reviewed

- Pinned source: `vendor/linux/include/net/tls_prot.h:1-68` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate snapshot: `rewrite/logs/tasks/S015671/candidate.diff`.
- Frozen records for S015671 in `SCOPE.tsv`, `SYMBOLS.tsv`, `ABI.tsv`, and
  `LIFETIMES.tsv`, plus every SC1 record in the task proposal.
- Pinned consumers: `net/handshake/alert.c:34-93`,
  `net/handshake/tlshd.c:452-455`, `net/sunrpc/svcsock.c:247-264`,
  `net/sunrpc/xprtsock.c:365-382`, `net/tls/tls.h:351-379`, and the trace
  consumer `include/trace/events/handshake.h:13-88`.

## Audit

The source declares three anonymous, untagged enum lists only.  It declares
no object, named enum type, structure, function, function-pointer prototype,
linkage symbol, include dependency needed by the declarations, or conditional
feature branch other than the conventional include guard.  Therefore those
anonymous lists have no separately instantiable or externally exposed layout,
alignment, ownership, locking, RCU, refcount, or lifetime contract.

The candidate preserves all 36 externally named enumeration constants,
spelling, and values: record content types 20..26; alert levels 1 and 2; and
all 27 alert descriptions with the original non-contiguous values through 120.
It also keeps their signed 32-bit integral-constant representation (`i32`),
which preserves the in-range values and signed integer use visible in the
pinned consumers.  Those consumers either initialize or compare against
`u8` record/control data, and the candidate changes none of those values.
The trace header consumes the same named values for symbolic trace metadata;
there is no candidate omission or renamed constant.

The conventional C include guard has no Rust run-time or exported ABI
equivalent.  No configuration conditional occurs in the source, and the
candidate’s provenance exactly matches task, Linux path, revision, SPDX
expression, and the common architecture scope.  No branding, stub, test,
unsafe code, allocation, synchronization, error path, or mechanism change is
present.

All proposed SC1 records are supported by this source review; no SC1 finding
is raised.
