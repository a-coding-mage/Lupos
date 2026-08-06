# Parity review — S016427 (slot 1)

## Result

ACCEPT.  No parity findings.

## Evidence reviewed

- Pinned source: `vendor/linux/include/uapi/linux/tty.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, complete lines 1–46.
- Candidate: `src/include/uapi/linux/tty.rs`, complete lines 1–44.
- Frozen task and scope rows identify this as common `RUST_TRANSLATE`, with
  both x86_64 and aarch64 selected through `metadata/header_closure.tsv`.
- `SYMBOLS.tsv` records the same non-configured definitions for both targets.
  There are no functions, types, storage, layout, linkage, locking, lifetime,
  or error-path semantics in this header.

## Exact macro audit

All 32 public macro spellings and values match exactly.  The candidate has 32
public constants, corresponding one-for-one to the 31 line-discipline IDs and
the limit:

`N_TTY=0`, `N_SLIP=1`, `N_MOUSE=2`, `N_PPP=3`, `N_STRIP=4`, `N_AX25=5`,
`N_X25=6`, `N_6PACK=7`, `N_MASC=8`, `N_R3964=9`,
`N_PROFIBUS_FDL=10`, `N_IRDA=11`, `N_SMSBLOCK=12`, `N_HDLC=13`,
`N_SYNC_PPP=14`, `N_HCI=15`, `N_GIGASET_M101=16`, `N_SLCAN=17`,
`N_PPS=18`, `N_V253=19`, `N_CAIF=20`, `N_GSM0710=21`, `N_TI_WL=22`,
`N_TRACESINK=23`, `N_TRACEROUTER=24`, `N_NCI=25`, `N_SPEAKUP=26`,
`N_NULL=27`, `N_MCTP=28`, `N_DEVELOPMENT=29`, `N_CAN327=30`, and
`NR_LDISCS=31`.

The C replacements are unsuffixed decimal integer literals, so each has the
C `int` literal category for these small values.  The candidate deliberately
uses `core::ffi::c_int` for every constant, preserving the signed C-int ABI
category on both frozen Linux targets.  The pinned UAPI integer definitions
also use signed `int` for `__s32` (`include/uapi/asm-generic/int-ll64.h:26`).

## Conditional and UAPI semantics

The only preprocessor conditional is the conventional
`_UAPI_LINUX_TTY_H` include guard (source lines 2–3 and 46); it is not
configuration-dependent and has no line-discipline value or externally linked
object semantics.  The Rust module needs no duplicate-definition guard.  All
32 value-bearing names remain `pub`, with no aliases, renames, feature gates,
or architecture-specific changes.  `NR_LDISCS` remains one greater than the
newest line-discipline identifier, preserving the array/range-bound contract
used by the pinned tty code.

## Provenance and rejection checks

The candidate preserves the upstream SPDX expression, exact Linux source path,
pinned revision, `common` architecture membership, and task ID.  The upstream
file has no additional copyright notice to retain.  No unauthorized branding,
test configuration, test, placeholder, panic, fake-success path, FFI layout,
or unreviewed behavior appears in the candidate.

No build, test, formatter, compiler, linker, emulator, debugger, or benchmark
was run for this review.
