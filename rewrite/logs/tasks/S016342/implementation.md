# Implementation evidence — S016342

- Task: `S016342`
- Linux source: `vendor/linux/include/uapi/linux/psample.h`
- Destination: `src/include/uapi/linux/psample.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (selected by both frozen x86_64 and AArch64 configurations)
- Branch and lease were verified before editing; the queue row is leased to `P02`.

The complete pinned header was translated. The anonymous attribute enum is represented by `i32` constants, matching the C enum constants used by the netlink ABI. The two named C enums are `#[repr(i32)]` Rust enums with every enumerator and explicit discriminant retained. The `PSAMPLE_ATTR_MAX` expression is retained as `__PSAMPLE_ATTR_MAX - 1`, and all four string/version macros are retained with their exact values. The include guard has no Rust runtime equivalent and introduces no selected symbol.

Required context read from the pinned tree included `include/net/psample.h`, the psample implementation and its Kbuild/Kconfig entries, `net/sched/act_sample.c`, and `include/uapi/linux/tc_act/tc_sample.h`; callers use the constants as netlink attribute and command IDs. No configuration branch changes the UAPI declaration.

No historical Lupos source, compiler, formatter, linker, test, runtime, or Git mutation was used.
