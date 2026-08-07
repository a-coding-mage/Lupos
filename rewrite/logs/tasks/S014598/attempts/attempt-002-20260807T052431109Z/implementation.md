# S014598 implementation attempt 2

- Source: `vendor/linux/include/linux/pci_ids.h` (pinned SHA `425f94c2954b1fe80ebdbf9b29854e89750355df`).
- Destination: `src/include/linux/pci_ids.rs`.
- Architectures: `x86_64,aarch64` (queue membership `common`).
- Scope: all 2,902 operative `#define` entries from the complete pinned header; the `_LINUX_PCI_IDS_H` include guard is represented by the Rust module boundary.
- Translation: each numeric unsuffixed C integer macro is preserved as a public `i32` constant. All values fit the C unsuffixed `int` rank; hexadecimal spellings and names are retained exactly. Inline source comments were non-operative and omitted from expressions.
- Evidence: pinned source lines 10-3270; scope row S014598; symbols rows for S014598; ABI/LIFETIMES records are closed by the semantic-closure proposal.
- No conditional branches beyond the source include guard are present in the pinned header.
