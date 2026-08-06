# Resolution — S000071

**Disposition: BLOCKED.**  The candidate is rejected and remains only as the
implementation-stage artifact; no source change can establish the required
parity within this frozen one-file Rust task.

## Finding P1 — `__ASSEMBLER__` branch

**Accepted.** `arch/arm64/include/asm/gpr-num.h:5-12` emits raw assembler
directives at preprocessing inclusion time.  This is operative: the assembler
branch of `asm/sysreg.h:1111-1121` consumes the `.L__gpr_num_\\rt` symbols, and
ARM64 assembly sources including that path invoke `mrs_s`/`msr_s` (for example
`arch/arm64/kernel/head.S:317` and `arch/arm64/mm/proc.S:169`).  The required
definitions therefore have to reach the assembler preprocessing stream before
those macro invocations.  A Rust module item cannot be included by that stream
or emit unquoted directives at that point.

The frozen task maps the header only to
`src/arch/arm64/include/asm/gpr-num.rs`; no provided source mapping specifies
how retained Linux assembly obtains a generated/preprocessed assembler header
from that Rust file.  Adding an assembly include, a build-generation rule, or
an assembler-facing compatibility artifact would be new cross-task/build
integration work outside S000071 and is not established by the frozen
manifests.

## Finding P2 / R1 — C preprocessor string-literal composition

**Accepted.** In the non-assembler branch,
`__DEFINE_ASM_GPR_NUMS` is an object-like C preprocessor macro whose adjacent
string-literal replacement tokens compose directly with each consumer's
template.  The direct uses include `asm/sysreg.h:1135-1159`,
`asm/asm-extable.h:97-133`, `asm/fpsimd.h:688-716`, and
`arch/arm64/kvm/pauth.c:22-29`.  Those contexts add assembler macro parameters,
C stringification, and/or inline-assembly operands at the same expansion
point.

The candidate `pub const __DEFINE_ASM_GPR_NUMS: &str` retains the directive
bytes but changes the interface to a Rust value.  It cannot stand in for
preprocessor tokens or preserve per-use composition, and no frozen macro
mapping or translated-consumer interface is provided that can supply an
equivalent compile-time assembly-template expansion.  Replacing it with a
`const` compatibility substitute would therefore be an intentional semantic
difference and is rejected.

## Blocking condition

Exact preservation requires a defined, audited mapping for both (1) the
assembler-preprocessor inclusion path used by retained ARM64 assembly and (2)
token-level assembly-template composition at each translated C/Rust consumer.
Neither mapping is supplied by the S000071 one-file task or frozen source
manifests.  Determining and introducing it requires a scope/ABI/build mapping
beyond this task.  The task is blocked rather than guessing or retaining the
Rust constant as a substitute.

No compiler, formatter, linker, test, runtime command, or diagnostic was used.
