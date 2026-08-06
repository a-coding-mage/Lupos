# Implementation — S012594

- Leased destination: `src/include/asm-generic/trace_clock.rs`.
- Pinned source: `vendor/linux/include/asm-generic/trace_clock.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen selection: aarch64 only; the scope record identifies this header as
  the generic resolved trace-clock header with 5,089 recorded consumers.
- The source's sole operative item is the default object-like
  `ARCH_TRACE_CLOCKS` macro.  Its guarded definition has an empty replacement
  list.  The Rust macro expands to no tokens, retaining the direct
  `trace_clocks[]` initializer context in `kernel/trace/trace.c`.
- C include guards are represented by Rust module loading and are therefore
  not translated into an additional runtime or data item.
- No architecture override, clock-function declaration, allocation, state, or
  side effect belongs to this generic header.
