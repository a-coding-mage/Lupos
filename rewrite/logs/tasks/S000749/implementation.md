# Implementation — S000749

- Task: `S000749`
- Pipeline/attempt: `P01` / `1`
- Linux source: `vendor/linux/arch/x86/include/asm/vermagic.h`
- Destination: `src/arch/x86/include/asm/vermagic.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `x86_64`
- Frozen configuration evidence: `rewrite/configs/x86_64/frozen.config` contains `CONFIG_X86_64=y` and does not enable `CONFIG_X86_32`.

## Source mapping

The complete pinned header was read. Its `CONFIG_X86_64` arm intentionally
does not define `MODULE_PROC_FAMILY`, and its final `CONFIG_X86_32` conditional
therefore selects `MODULE_ARCH_VERMAGIC` as the empty string for this frozen
x86_64 task. The non-x86_64 processor-family arms and the `CONFIG_X86_32`
assignment are not active in the approved architecture/configuration and are
not represented as Rust feature guesses. The Rust destination exposes the
selected string value as `MODULE_ARCH_VERMAGIC`.

No unsafe operation, allocation, cleanup, synchronization, or test code is
needed for this header-only constant mapping.
