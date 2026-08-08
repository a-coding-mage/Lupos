Task S000758 implementation record

Source: vendor/linux/arch/x86/include/asm/vmxfeatures.h
Destination: src/arch/x86/include/asm/vmxfeatures.rs
Linux revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
Architecture: x86_64

Translated the complete pinned header: NVMXINTS and all VMX_FEATURE_* operative
macros, retaining each C constant expression as an explicitly signed i32 Rust
constant. The include guard maps to the Rust module boundary; no configuration
branches beyond the unconditional pinned header were present. Direct context
checked: asm/vmx.h, asm/processor.h, arch/x86/kernel/cpu/feat_ctl.c, and
arch/x86/kernel/cpu/proc.c. No unsafe code, tests, stubs, or generated indexes
were added.

Frozen input hashes:
identity 0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2
queue cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f
scope b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a
symbols 7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf
ABI ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39
lifetimes 0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8
