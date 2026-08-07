# Parity review — S016105, slot 1, attempt 2

Reviewed `vendor/linux/include/uapi/linux/dpll.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/dpll.rs`, restricted to source evidence.  No compiler,
formatter, linker, test, diagnostic, or historical translation was used.

Sealed semantic proposal: `a652994449e0edd27a49f057d0f176248aeec6f3bf744a3859d7782a6c32a758`
(817 records).  Its candidate binding is the current `candidate.diff` digest
`b4175ae7918a3e1edf0f98689bcf0067c9be455b15b25f47b511a529ec007227`;
the current destination digest is
`76f2e5e8089f528f363cc66814d3cf4fb2c31e692dfd894c717393b015283609`.

## Finding P1 — source SPDX identifier is not retained

`dpll.h:1` declares `((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)`.
The destination instead declares `GPL-2.0-only`.  This changes the upstream
licensing/provenance identifier and is not an allowlisted branding delta.
Restore the exact source SPDX expression in the Rust file before acceptance.
This affects whole-file semantic closure and is attached to proposal key
`SC1-da57082966bf194f0bc491eb57e798e85e58a9c79ea3b497f81d25fecc5a694c`.

## Finding P2 — selected include-guard macro has no destination representation

`dpll.h:7-8` conditionally defines `_UAPI_LINUX_DPLL_H`; `dpll.h:310` closes
that conditional.  The candidate has no same-named declaration or documented
equivalent, while the sealed proposal marks the guard's selection expression
as reviewed for both frozen architectures.  The translation therefore neither
preserves nor explains the selected preprocessor guard/name.  Supply a
source-faithful Rust-side representation, or document and implement the exact
module-level mechanism that provides the guard's observable one-inclusion
contract, then reconcile the two guard proposal records.

Proposal keys: `SC1-e9d27ac340951e97825ac7d3a529eaa71df7af21211693e09c7c80714f350f1e`,
`SC1-ae45b88baba76632c42b2c6825e74042083bfb8c64cd13089a8d75c7622c0cd1`,
`SC1-2e7a98b20cde53ff77d0ebbb7328497226265e26458e73daf2faddfe8eb176fa`,
and `SC1-7f8c10e7c284d20a9ae22bbd4a488cacd2b3692b2662da16fe1e9841c71e379d`.

Apart from those findings, manual comparison found all fourteen enum domains,
their explicit/implicit ordinal sequences and maxima, all numeric macro
values, both string payloads and NUL extents, and all public names present
with matching values.  The header defines no bitfields, layouts, functions,
or configuration-selected feature branches beyond the include guard.

Result: FINDINGS.  Reviewer attribution: parity_reviewer, gpt-5.6-terra,
high.
