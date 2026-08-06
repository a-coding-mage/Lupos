# S000013 implementation

Source: `vendor/linux/arch/arm64/include/asm/acenv.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete upstream header is an unconditional include guard with no body:
it declares no macro value, type, function, static, ABI object, or
architecture-specific ACPICA override. The frozen AArch64 configuration has
`CONFIG_ACPI=y`; `include/acpi/platform/aclinux.h` includes this header under
that condition, and Phase 0 records 2,304 consumers. The guard prevents only
multiple textual C inclusion within an individual translation unit.

`src/arch/arm64/include/asm/acenv.rs` is consequently an intentionally empty
Rust module. Rust module declaration/loading provides the corresponding
single-definition property without inventing a Rust representation of the C
preprocessor guard. The nearby ACPICA Linux platform header supplies the
actual ACPI environment definitions; none are supplied by this ARM64 header.

No ownership, lifetime, layout, linkage, synchronization, or error behavior
is present in the source. No compiler, formatter, build, test, or runtime tool
was used.
