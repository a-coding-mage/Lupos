# Implementation — S012494

Translated `include/acpi/proc_cap_intel.h` to `src/include/acpi/proc_cap_intel.rs` from pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected x86_64 header has no functions, types, storage, includes, or configuration branches. Its twelve base capability masks and three composite capability masks are represented as public `u32` constants. The selected x86 callers operate on `u32` capability buffers, so the explicit Rust width preserves the use-site bitwise semantics.

No build, formatter, test, or runtime command was run.
