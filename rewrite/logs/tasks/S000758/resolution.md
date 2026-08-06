# Resolution — S000758

Applier reopened the complete pinned
`vendor/linux/arch/x86/include/asm/vmxfeatures.h` and its selected consumers:
`arch/x86/kernel/cpu/mkcapflags.sh`, `arch/x86/kernel/cpu/proc.c`,
`arch/x86/kernel/cpu/feat_ctl.c`, `arch/x86/include/asm/processor.h`, and
`arch/x86/include/asm/vmx.h`.

## P1 — signed C macro expressions

**Accepted and fixed.** `NVMXINTS` and all 64 `VMX_FEATURE_*` constants now
use `i32`.  The upstream definitions use only unsuffixed decimal literals, so
each source expression is a signed C `int` on frozen x86_64.  The original
`word * 32 + bit` expressions, all indices, `NVMXINTS = 5`, and the maximum
index (`VMX_FEATURE_IPI_VIRT = 100`) are retained.  Rust consumers that need a
collection index or unsigned shift must make that conversion at their use
site, just as the Linux consumers impose their own contextual conversion.

The Rust review's conclusion that the previous `u32` values could not change
the represented numerical values is noted, but it does not override the
upstream signed-expression contract required by the parity finding.

## P2 — quoted VMX flag names

**Accepted and fixed.** The frozen configuration enables
`CONFIG_X86_VMX_FEATURE_NAMES`.  Upstream `mkcapflags.sh` scans this header
and creates `x86_vmx_flags[NVMXINTS * 32]`, inserting a string only when a
comment begins with a quoted string; `proc.c` emits non-null selected entries
as the `vmx flags` field.  `VMX_FEATURE_NAMES` now has exactly the equivalent
160 index slots and records all 29 upstream quoted names at their original
feature indices.  Every other entry is `None`, representing the corresponding
generated C null pointer and preserving the distinction from a quoted name.

No source symbols, ABI layout, ownership, synchronization, or unsafe contract
exists in this macro header beyond these compile-time index and metadata
contracts.  No build, compiler, formatter, test, runtime, or benchmark command
was run.
