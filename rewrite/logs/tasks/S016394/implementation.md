# Implementation evidence — S016394

- Task: `S016394`, pipeline `P01`, attempt `1`.
- Pinned source: `vendor/linux/include/uapi/linux/sunrpc/debug.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/include/uapi/linux/sunrpc/debug.rs`.
- Architectures: `common` (the frozen x86_64/AArch64 union).
- The complete pinned header was read. The Rust file preserves the SPDX notice, copyright/provenance, RPC debug mask values, and all eight anonymous-enum sysctl values.
- C object-like macros are represented as typed `i32` constants; the anonymous C enum's enumerators are likewise `i32` constants, preserving the C `int` value domain and exact values.
- No conditional source branches occur beyond the C include guard; Rust module loading supplies the same single-definition boundary.
- No compiler, formatter, test, runtime, Git mutation, or historical Lupos source was used.
