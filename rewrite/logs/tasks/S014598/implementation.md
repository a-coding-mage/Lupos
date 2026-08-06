# S014598 implementation evidence

- Task: `S014598` (`include/linux/pci_ids.h` -> `src/include/linux/pci_ids.rs`)
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: x86_64 and aarch64 (the frozen task row classifies the header as `common`).
- Implementer model/effort: `gpt-5.6-terra` / `medium` (configured Luna fallback unavailable).

The complete 3,270-line pinned header was read.  It contains no function-like
macros and no selected configuration branches beyond its conventional include
guard.  Its 2,902 numeric object-like PCI class, vendor, device, subsystem, and
subvendor ID macros were converted one-for-one, with their original spelling
and hexadecimal value, to public `u32` constants.  `u32` preserves every
unsigned PCI identifier and class value represented by the C literals.

The C include guard (`_LINUX_PCI_IDS_H`) is intentionally represented by the
Rust module boundary rather than as a Rust item; it has no identifier/value
semantics after Rust module inclusion.  No other source definitions remain
unmapped.  A sorted name/value comparison of the pinned numeric macro set with
the generated Rust constants reported no differences (2,902 on each side).

No compiler, formatter, linker, test, or runtime command was run.
