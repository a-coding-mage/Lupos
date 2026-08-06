# Rust review — S012494 (slot 2)

## Scope reviewed

- Pinned source: `vendor/linux/include/acpi/proc_cap_intel.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Candidate: `src/include/acpi/proc_cap_intel.rs`.
- Frozen scope: x86_64 header task S012494; the selected inventory contains the include guard, twelve base capability macros, and three composite macros.
- Relevant x86 consumer: `vendor/linux/arch/x86/include/asm/acpi.h`, where `arch_acpi_set_proc_cap_bits(u32 *cap)` ORs the masks into and clears masks from the `u32` processor-capability buffer. `arch/x86/xen/enlighten_pv.c` likewise places the composite value in a `u32` buffer.

## Rust/ABI audit

The candidate declares every base mask as a public `u32` constant with the exact source value: `0x0001`, `0x0002`, `0x0004`, `0x0008`, `0x0010`, `0x0020`, `0x0040`, `0x0080`, `0x0100`, `0x0200`, `0x0800`, and `0x1000`. The three composite constants preserve the upstream OR operands and therefore yield the same masks (`0x000b`, `0x082b`, and `0x031a`).

Although the C hexadecimal literals have C `int` type before use-site promotion, each source value fits in a signed `int` and the operative x86 consumers use a `u32` capability word. Rust `u32` makes that destination width explicit. In the C clear path, C complements the signed composite expression and then converts it for `u32 &=`; Rust complements the same low 32-bit mask as `u32`. Both produce the same 32-bit result. No FFI item, layout-bearing type, pointer, ownership relationship, allocation, synchronization primitive, or unsafe operation exists in this header/candidate.

## Findings

None. The candidate is Rust-semantically suitable for the selected x86_64 uses and introduces no Rust ownership, layout, cast, panic, or unsafe-boundary defect.

No build, formatter, test, or runtime command was run.
