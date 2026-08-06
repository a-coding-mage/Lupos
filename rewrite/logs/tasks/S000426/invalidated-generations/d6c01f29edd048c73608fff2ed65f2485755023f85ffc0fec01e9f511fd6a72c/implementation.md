# S000426 implementation

Translated `arch/x86/events/amd/iommu.h` to
`src/arch/x86/events/amd/iommu_h.rs` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` for the frozen x86_64 selection.

The complete upstream header contains only its include guard and eight
object-like macros.  The include guard has no runtime or Rust module analogue.
Each macro is represented by a public Rust constant with its unchanged name
and integer value.  Unsuffixed literals in the frozen C target are signed
`int`, so each constant is `i32`; consumers that call the AMD IOMMU interface
whose `fxn` argument is `u8` retain that C conversion at their call boundary.

Read context: the complete AMD IOMMU PMU implementation
`arch/x86/events/amd/iommu.c`, its Kbuild selection in
`arch/x86/events/amd/Makefile`, frozen `CONFIG_CPU_SUP_AMD=y` and
`CONFIG_AMD_IOMMU=y`, the AMD IOMMU public function declarations in
`include/linux/amd-iommu.h`, and their original driver-object implementations
in `drivers/iommu/amd/init.c`.  The PMU uses the six register constants only
as the `u8` function selector passed to `amd_iommu_pc_{get,set}_reg`; the two
limit macros have no additional selected consumer in the pinned tree.

No compilation, formatter, test, runtime command, or historical Rust source
was used.
