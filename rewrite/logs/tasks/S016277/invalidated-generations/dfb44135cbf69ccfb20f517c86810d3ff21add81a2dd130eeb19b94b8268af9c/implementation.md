# S016277 implementation

- Source: `vendor/linux/include/uapi/linux/netfilter/nf_tables.h`
- Revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/uapi/linux/netfilter/nf_tables.rs`
- Scope: all selected common-architecture UAPI enum declarations, enumerators, and object-like macros.
- Translation: generated directly from the complete pinned header; C enum values are represented as `i32` type aliases and constants, preserving numeric values and macro expressions.
- Semantic decisions: UAPI declarations have no runtime ownership, storage, locking, or calling convention; ABI is integer constants/types only. Source citations are recorded per closure field.
