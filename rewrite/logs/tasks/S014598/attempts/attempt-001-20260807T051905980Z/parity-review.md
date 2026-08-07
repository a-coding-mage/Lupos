# Parity review — S014598

Reviewed only the pinned `vendor/linux/include/linux/pci_ids.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate
`src/include/linux/pci_ids.rs`, frozen task/manifests, relevant pinned PCI
header callers, and the sealed semantic-closure proposal.  No build, test,
formatter, compiler, or compiler-backed diagnostic was run.

## Inventory result

The source has 2,902 non-guard `#define` ID/class macros.  The candidate has
2,902 `pub const` declarations.  Manual text comparison of identifier and
numeric replacement value found no missing, extra, or value-mismatched
non-guard definition.  Every upstream non-guard replacement is a plain
unsuffixed hexadecimal literal of at most six hex digits; every candidate
declaration is explicitly `u32`.  No configuration conditional occurs inside
the header other than its include guard.

## Findings

### PARITY-001 — `_LINUX_PCI_IDS_H` include-guard macro and branch are omitted

Pinned Linux symbol/branch: `_LINUX_PCI_IDS_H`, the `#ifndef` at
`vendor/linux/include/linux/pci_ids.h:10`, its `#define` at line 11, and the
matching `#endif` at line 3270.

Local evidence: frozen `SYMBOLS.tsv` selected the guard macro and both guard
conditionals for x86_64 and AArch64.  The sealed proposal marks their
selection/status records `selected`/`COMPLETE` (record keys
`SC1-8fe5caf89d71ced9219128329ea229853bc263ddb2d5a0d5e425667b5c5474c0`,
`SC1-0fdf4131048c9d0958448dde7296f06bc7ad669340244d5d89519df29d80d1e9`,
`SC1-c9bbd44067140d83674a52fea7b537e89d9d4adb3ed615989b7cb531ba83ec67`,
`SC1-05bed97add58c3f728628cd5fb2a272faac0dce438a738c9c04f80fa6a93eacc`,
`SC1-2e16f191c3a8d02e4dd25f5323173a400157fc7cb539f2a08ced3ca1eb9c42db`,
`SC1-1f998305dd4438e313152f589d575009569fe52080cab8447cb3770d5f8eb4bd`,
`SC1-af8d9b3887a5b15a289da00052bba2aba8cadb1936c923c7a83bd1bfe01414ee`,
and `SC1-245c6d13a18cb884d536cfd59b747f34570ef76d5307fc09d31a9d6eff9c583d`).
The candidate has no corresponding definition or conditional; its first
operative item is a Rust `pub const` at line 19.

This changes the selected compile-time include/re-inclusion behavior rather
than mapping it.  Rust module loading may make an include guard unnecessary,
but no frozen local evidence establishes that as an equivalent mechanism for
both selected architectures.  The proposal's `COMPLETE` decisions therefore
need source-backed adjudication before closure.

### PARITY-002 — all PCI ID/class macros lose C `int` replacement semantics

Pinned Linux symbols: all 2,902 non-guard macros, for example
`PCI_CLASS_NOT_DEFINED` at `vendor/linux/include/linux/pci_ids.h:15`,
`PCI_CLASS_STORAGE_SATA_AHCI` at line 25, and `PCI_VENDOR_ID_INTEL` at line
2690.  Their unsuffixed literals are at most six hex digits, hence are
`int`-typed C integer constants on the pinned 32-bit-`int` Linux targets.

Local evidence: the candidate declares every corresponding item as `u32`,
starting with `PCI_CLASS_NOT_DEFINED: u32` at
`src/include/linux/pci_ids.rs:19`.  The relevant pinned consumer header
`vendor/linux/include/linux/pci.h:362-366` holds vendor/device/subsystem IDs
as `unsigned short` and class as `unsigned int`; its `pci_is_vga()` compares
the class word with these macros at lines 784-791.  The C replacement tokens
therefore undergo the ordinary C promotions/conversions required by each use;
a monomorphic Rust `u32` does not preserve that contextual `int` behavior and
requires a different downstream conversion discipline.

The sealed proposal has no field that records or justifies the macro
replacement type/form.  Its 11,617 records are internally complete, unique,
and cite the pinned header, but their `selected`/`COMPLETE` conclusions cover
selection/status only.  They cannot close this sign/width/promotion difference
without a source-backed mapping decision.

## Conclusion

Do not approve parity as-is.  No numeric identifier/value omission was found,
but PARITY-001 and PARITY-002 require applier disposition from pinned local
source and frozen guidance.
