# Rust semantic review — S000191

Reviewer: `rust_reviewer`  
Pipeline: `P01`  
Scope: `arch/arm64/include/asm/vdso.h` -> `src/arch/arm64/include/asm/vdso.rs`  
Method: independent manual source inspection only; no compiler, formatter, test, or Rust-analyzer diagnostics used.

## Result: findings required

### R1 — zero-sized foreign statics do not faithfully model the linker-owned byte anchors

**Severity:** high  
**Candidate:** `src/arch/arm64/include/asm/vdso.rs:20-28,33-52`  
**Linux evidence:** `vendor/linux/arch/arm64/include/asm/vdso.h:19-20`; the actual definitions are page-aligned linker labels around non-empty `.incbin` images in `vendor/linux/arch/arm64/kernel/vdso-wrap.S:14-21` and `vendor/linux/arch/arm64/kernel/vdso32-wrap.S:13-20`.

The candidate declares every imported label as `static ...: [u8; 0]` and then returns a pointer derived from that zero-sized static.  The C declarations are incomplete arrays of `char`, whose identifier expression is an address of linker-owned byte storage.  In particular, `vdso_start` and `vdso32_start` delimit images subsequently read by consumers: `vdso.c:48-49,76-78` validates and page-walks the image, and `alternative.c:213-220` parses the ELF image through `vdso_start`.

An address formed from a Rust ZST static does not establish a byte object or an extent usable as the vDSO image.  The comment's claim that a zero-length anchor preserves the C address-only contract is therefore not established: downstream raw-pointer reads would originate from a ZST declaration rather than a declared byte anchor.  This is an ownership/provenance mismatch even though the linker relocation names happen to be correct.

**Required resolution:** declare each linker symbol as an external, address-only non-ZST byte anchor (for this frozen command family `char` is unsigned; the aarch64 command evidence includes `-funsigned-char`), take only its raw address, and clearly document that the assembly objects S000292/S000294 own the storage.  Do not form Rust references or dereference these declarations in this header.  Preserve the four exact linker names `vdso_start`, `vdso_end`, `vdso32_start`, and `vdso32_end` via the imported symbol names/link names.

### R2 — wrapper pointer constness and type contract narrow the C declarations

**Severity:** medium  
**Candidate:** `src/arch/arm64/include/asm/vdso.rs:33-52,60-61`  
**Linux evidence:** `vendor/linux/arch/arm64/include/asm/vdso.h:19-20` declares `extern char ...[]`, not `const char ...[]`; `vendor/linux/arch/arm64/kernel/signal.c:1484` passes the mutable `mm->context.vdso` value to `VDSO_SYMBOL`; the macro returns `(void *)` at `vdso.h:14-17`.

The candidate exposes all four array identifiers only through functions returning `*const u8`, then gives the macro helper a `*const u8` input.  That is a stricter, different interface from the C header's mutable `char *` array decay and `void *` expression.  It forces later translations to introduce casts merely to reproduce legal header-level uses and obscures the exact raw-pointer contract.  The memory happens to be placed in `.rodata`, but that is not a license to change the declared C type in the shared header.

**Required resolution:** retain a mutable raw byte/address interface for the declarations and macro input/result at this boundary (or give a documented direct equivalent that preserves C `char *`/`void *` convertibility without creating references).  Read-only consumers can make their own const conversion, as Linux does in `vdso_abi_info`.

### R3 — generated offset binding is not represented, and the Rust macro changes the source macro contract

**Severity:** high  
**Candidate:** `src/arch/arm64/include/asm/vdso.rs:55-72`  
**Linux evidence:** `vdso.h:11,14-17` includes `generated/vdso-offsets.h` and token-pastes `vdso_offset_##name`; `vendor/linux/arch/arm64/kernel/vdso/gen_vdso_offsets.sh:12-16` generates `#define vdso_offset_<name> 0x...`; `rewrite/metadata/header_include_edges.tsv:605` and `header_closure.tsv:3348` record the selected generated header as BUILD_METADATA with four consumers.  The frozen configuration enables both `CONFIG_COMPAT=y` and `CONFIG_COMPAT_VDSO=y` (`rewrite/configs/aarch64/frozen.config:498-500`).

No generated-offset Rust binding is imported or otherwise made available by this file.  Instead, `VDSO_SYMBOL!` accepts an arbitrary `$offset:expr`; unlike the C `name` argument, that expression can have side effects and is evaluated after `$base` under Rust's left-to-right call argument evaluation.  In C, `name` is a preprocessor token used only to select a generated constant, so it has no runtime evaluation, while the only runtime operand is `base`.  The candidate also changes the public use shape from `VDSO_SYMBOL(base, sigtramp)` to an unspecified explicit integer argument.  Its documentation claims a binding has supplied the offset but no such binding is present in the translation tree.

The arithmetic itself correctly selects `usize`/wrapping addition for the frozen AArch64 `unsigned long` expression, provided the generated offset is a `usize` constant.  That conditional fact does not repair the missing generated-binding/one-evaluation contract.

**Required resolution:** provide a deterministic Rust representation of the selected generated `vdso_offset_<name>` constants, tied to the frozen BUILD_METADATA artifact, and make the call interface preserve the C macro's identifier-to-constant behavior (or record and apply the unavoidable Rust mapping in the file map/ABI evidence).  The final expansion must evaluate the runtime base exactly once and must not introduce a second, side-effect-capable runtime operand.  Keep arithmetic explicitly 64-bit unsigned and wrapping for the frozen `aarch64-linux-gnu` target.

### R4 — unsafe FFI boundary has no local safety invariant

**Severity:** low  
**Candidate:** `src/arch/arm64/include/asm/vdso.rs:20-28`  
**Linux evidence:** linker definitions above; scope classifies `vdso-wrap.S` and `vdso32-wrap.S` as `LINUX_ARCH_ASM` (`rewrite/SCOPE.tsv`, rows S000292 and S000294).

`unsafe extern "C"` imports four storage symbols, but there is no local `// SAFETY:` statement recording the required invariant: the preserved assembly objects define the exact labels for the enabled configuration; this Rust header only obtains their addresses and creates no references/dereferences.  The surrounding prose is not an unsafe-boundary invariant and, as written, incorrectly relies on ZST anchors.

**Required resolution:** after correcting R1, add a local safety comment at the narrow FFI declaration/address-formation boundary naming the assembly owner, exact symbols, static lifetime, and address-only use.

## Verified observations and pending-record closure requirements

- `__VDSO_PAGES` is `4` in `vdso.h:8`; the candidate's value is correct for this configuration.  It is a source constant, not a runtime allocation claim.
- The source header's `__ASSEMBLER__` exclusion needs no Rust `cfg`: this Rust module is the non-assembly consumer counterpart.  The required assembly definitions remain original LINUX_ARCH_ASM objects, not Rust translations.
- The frozen configuration has `CONFIG_COMPAT_VDSO=y`, so the `vdso32_*` labels are active and must not be conditionally omitted.  The candidate's unconditional presence is appropriate for this frozen config, subject to R1/R2.
- The imported linker symbol spelling is correctly requested with `#[link_name]`, but the Rust private anchor identifiers are not a substitute for documenting/completing the ABI record for all four C array declarations.
- For `SYMBOLS.tsv` S000191: close `__VDSO_PAGES` as a selected integer constant; close `VDSO_SYMBOL` as a generated-offset, single-runtime-base expression; close both guard records as source-header inclusion mechanics; and record all four array labels, including the parser-collapsed starts, as selected external address symbols.
- For `LIFETIMES.tsv` S000191: `vdso_start`, `vdso_end`, `vdso32_start`, and `vdso32_end` have static/link lifetime owned by the preserved assembly image wrappers.  No Rust owner, borrow, reference, Drop action, lock, RCU, or refcount applies; consumers may retain only raw addresses under the normal kernel image lifetime.
- For `ABI.tsv` S000191: all four labels have exact external C/assembly linker names and `char[]` address semantics (unsigned byte under the frozen aarch64 C command).  They are not Rust-exported storage and have no Rust layout beyond the address anchor.  The macro's input/output is raw pointer/address arithmetic with `unsigned long` / `usize` wrapping semantics; no calling convention applies.

No compiler, linker, formatter, test, benchmark, emulator, debugger, or rust-analyzer diagnostic was run or used.
