# Implementation evidence

- Task: `S014648`
- Pipeline/attempt: `P02` / `1`
- Linux source: `vendor/linux/include/linux/pinctrl/pinctrl-state.h`
- Destination: `src/include/linux/pinctrl/pinctrl-state.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by both frozen x86_64 and AArch64 configurations)
- Source review: the complete pinned header was read. It contains only the include guard, documentation, and four object-like string macros; it declares no structs, enums, functions, storage, or conditional branches beyond the guard.
- Context review: the pinned `consumer.h` and `machine.h` include this header, and pinned consumers use the four names as exact state-name strings, including adjacent literal concatenation in diagnostics. The frozen symbol inventory records only the include-guard and four `PINCTRL_STATE_*` macros as operative symbols.
- Translation decision: represent the four macro values as public compile-time string constants with the exact spelling and case of the Linux literals. No runtime state, allocation, ownership, FFI layout, or unsafe boundary is introduced because the source header has none.
- Validation restriction: no compiler, formatter, linker, test, runtime, or analyzer command was run.
