# xtask

Use two public build commands:

```bash
cargo xtask build
cargo xtask build --userland --iso
```

`cargo xtask build` builds the pure Lupos kernel: ELF plus bzImage, with no
userland and no ISO. `cargo xtask build --userland --iso` stages the minimal
Arch `base` userland and emits the bootable ISO on the same generic x86_64
config.

The top-level Makefile equivalents are `make kernel` and `make image`; plain
`make` is the image path.

Use `test` and `run` for validation and QEMU boots:

```bash
cargo xtask test
cargo xtask run
```

Flags select specialized paths:

```bash
cargo xtask build --modules
cargo xtask test --mode module-loader
cargo xtask test --mode userspace-smoke
cargo xtask test --mode runtime-stress
cargo xtask test --all
cargo xtask run --headless
cargo xtask run --ping-smoke
```

The default `cargo xtask test` path includes the critical runtime parity gate:
`audit-parity --require-tags --fail-on stub --scope critical-runtime`.

Older verbs such as `test-boot`, `audit-layout`, `modules`, and `suite-*`
remain backend commands for scripts and focused debugging, but new docs and
workflows should use `build`, `test`, and `run`.
