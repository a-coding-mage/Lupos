# Rust review — S016150 / attempt 1 / P02

Reviewer: `rust_reviewer` (`gpt-5.6-terra`, high).  This was a manual
source-only review of `vendor/linux/include/uapi/linux/hsr_netlink.h`, the
candidate, the candidate snapshot, and the frozen task records.  No compiler,
formatter, test, analyzer, or historical Lupos source was used.

## Findings

### R1 — the selected C include-guard contract has no demonstrated Rust mapping (blocking)

`__UAPI_HSR_NETLINK_H` at upstream lines 14–15 prevents repeated textual C
inclusion from redeclaring the two anonymous enums.  `SYMBOLS.tsv` records that
guard and both `#ifndef`/`#endif` conditionals as selected semantic records.
The candidate exposes constants but contains neither an equivalent module
single-definition mechanism nor a source-backed explanation of where the
module graph enforces the one-definition property.  A Rust source file alone is
not an equivalent to the C preprocessor guard: repeated Rust `mod` declarations
are an error, while repeated C inclusions are intentionally harmless.  The
pending guard record therefore cannot be closed merely by setting its proposal
field to `SOURCE_REVIEWED_VALUE`.

This is a compile-time UAPI/namespace semantic question, not a runtime
ownership or unsafe issue.  It requires a pinned-source and frozen-module-map
backed mapping before acceptance; do not invent a Rust macro or marker
constant, because that would not reproduce C preprocessor behavior.

Semantic-closure mapping: `SC1-fc590a192de2ca1b902b2560abc5338e79fc723d4c9b7e01cdeefda2a73393f6`,
`SC1-0c4b20a8025addc5dac2efe8154c59fa302802934c5d1ee1a57294600b4dc186`,
`SC1-7ba3752fe0d3945832e20610fc82d872438edc35dfaf7f0a23873ea4ab6340b2`.

### R2 — relevant upstream copyright notice was dropped (must fix before acceptance)

The source header retains the 2011–2013 Autronica Fire and Security AS
copyright and the named author immediately after the SPDX line.  The candidate
retains the SPDX identifier but omits that relevant upstream notice, contrary
to the required path-preserving source-tree rule.  Restore it as a Rust comment
without turning it into a behavioral claim.

The public constant namespace, ordinal values, and two `MAX` arithmetic
expressions otherwise match the header.  There are no Rust ownership,
aliasing, `unsafe`, `Drop`, layout, FFI-object, or panic paths in this
constant-only candidate.  The anonymous C enum declarations introduce no named
or passed UAPI object in the pinned header; the visible enum constants remain
within the signed C `int` value range, so this review does not independently
reject the `i32` values on that basis.

Semantic-closure mapping for the affected source-declaration set:
`SC1-cdb2f3f78417d0ad614119bb836ccf50358f8e8dca282883ccb07d18b0622299`,
`SC1-30e1c23caeb18163be152b833b81aec7b457cc87bbad5dc07a171d63c70c2c69`.

## Result

`FINDINGS`.  R1 prevents closing the selected guard semantic record; R2 also
requires a source-only correction and fresh candidate/review binding.
