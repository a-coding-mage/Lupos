# Canonical Phase 0 toolchain

The authoritative Phase 0 toolchain is the complete LLVM 19 suite under
`/usr/lib/llvm-19/bin/`. Every Kconfig, Kbuild, metadata, preparation, and
validation invocation uses the absolute value:

```text
LLVM=/usr/lib/llvm-19/bin/
LLVM_IAS=1
```

The selected linker is `/usr/lib/llvm-19/bin/ld.lld`, resolved to the LLVM 19
`lld` binary. Rust-distributed LLD 22 executables remain recorded in
`LINKER_INVENTORY.tsv` for rejection evidence and are never selected.
