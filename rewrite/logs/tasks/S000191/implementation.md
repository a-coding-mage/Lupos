# S000191 implementation

- Oracle: `vendor/linux/arch/arm64/include/asm/vdso.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Frozen architecture/configuration: `aarch64`; `CONFIG_COMPAT=y` and `CONFIG_COMPAT_VDSO=y`, so both native and AArch32 linker boundary pairs are retained.
- `__VDSO_PAGES` is represented as the same value, `4`.
- The four C incomplete `char` arrays are address-only linker symbols emitted by `vdso-wrap.S` and `vdso32-wrap.S`. They are represented as private zero-length foreign anchors and exported only through raw-address accessors, which create no Rust reference to linker-owned memory.
- The generated `vdso-offsets.h` provides per-symbol integer offsets. `vdso_symbol` performs the C macro's AArch64 unsigned-long address addition with explicit wrapping arithmetic; `VDSO_SYMBOL!` accepts that generated offset explicitly because Rust has no preprocessor token concatenation.
- No build, formatter, test, linker, compiler, or runtime command was run.
