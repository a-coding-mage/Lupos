# S000758 implementation attempt 2

decision: COMPLETE
final: SOURCE_REVIEWED_VALUE

Task identity was checked against the frozen queue: S000758, `P02`, attempt 2,
source `arch/x86/include/asm/vmxfeatures.h`, destination
`src/arch/x86/include/asm/vmxfeatures.rs`, architecture `x86_64`, Linux
revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete pinned header was translated. `NVMXINTS` and every selected
`VMX_FEATURE_*` macro are represented as public signed `i32` constants, while
retaining each upstream arithmetic expression and value. The include guard is
represented by the Rust module boundary. The source SPDX identifier is the
exact upstream `GPL-2.0` value.

Selected conditional branches: `#ifndef _ASM_X86_VMXFEATURES_H` and its
matching `#endif`; no configuration-dependent branches occur in the pinned
file. No callers, callees, types, or local headers add implementation beyond
this self-contained feature-bit header.

Source-review closure: COMPLETE for implementation content. Semantic status
fields remain `SOURCE_REVIEWED_VALUE` for this implementation record.
