# S000200 implementation record

- Leased task: `S000200` / `P02`; queue state was `IN_PROGRESS` with lease owner `p02-terra-fallback`.
- Pinned source: `arch/arm64/include/asm/vncr_mapping.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df` (SHA-256 `87882faa68e0cea46ad6a2e1cc1fa2d03a470b52a2f96e8cf864cab0c48ce3fd`).
- Frozen architecture: `aarch64`; `CONFIG_ARM64=y` and `CONFIG_KVM=y` in `rewrite/configs/aarch64/frozen.config`.
- Mapping: every one of the 104 unconditional C `VNCR_*` integer macros is a same-named public Rust `i32` constant with the identical byte displacement. The C literals fit and have `int` type; retaining `i32` preserves that source integer width while keeping byte-offset arithmetic explicit at use sites.
- Context: `arch/arm64/include/asm/kvm_host.h` divides these byte displacements by 8 while deriving VNCR sysreg-array enum values. No runtime state, allocation, pointer manipulation, conditional mapping, ABI object, or unsafe operation is present in this header.
- No build, formatter, test, or runtime command was run.
