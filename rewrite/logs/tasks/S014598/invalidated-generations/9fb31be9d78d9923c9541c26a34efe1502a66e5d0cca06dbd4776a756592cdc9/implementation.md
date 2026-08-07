# Implementation evidence

- Task: `S014598`
- Linux source: `vendor/linux/include/linux/pci_ids.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/linux/pci_ids.rs`
- Architectures: common (`x86_64,aarch64`)
- Scope class: `RUST_TRANSLATE`; low risk.

The complete 3270-line pinned header was read. It contains the include guard,
comments, and 2898 numeric `#define` entries (all hexadecimal literals).
The include guard directives are represented by the Rust module boundary. Each
numeric definition is preserved as a public `u32` constant with the original
identifier and literal value, including duplicate values and source ordering.
Trailing source comments are retained. No configuration-dependent branches,
types, callers, callees, or external macros occur in this header; its entries
are public PCI class/vendor/device identifiers consumed by PCI matching code.

The Rust file begins with the required immutable provenance and contains no
stubs, tests, panic paths, or branding changes. The conversion is mechanical:
`#define NAME 0xVALUE` becomes `pub const NAME: u32 = 0xVALUE;`.
