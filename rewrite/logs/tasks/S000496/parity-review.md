# Parity review — S000496, attempt 1, slot 1

Scope reviewed: `src/arch/x86/include/asm/cpufeatures.rs` against the pinned
`arch/x86/include/asm/cpufeatures.h`, the frozen x86_64 configuration and
task-local sealed semantic-closure proposal.  This was manual source review
only: no compiler, formatter, test, diagnostic service, Git command, source
edit, candidate-diff inspection, or historical Lupos source was used.

Preconditions observed: the checked-out ref is
`refs/heads/feat/bun-like-rewrite-test`; S000496 is `REVIEWING`, attempt `1`,
pipeline `P02`; the pinned Linux revision is
`425f94c2954b1fe80ebdbf9b29854e89750355df`; and the frozen x86_64 identity
records `x86_64-linux-gnu`, config digest
`a1cdb40573726de54a174da53c2eac8811dd84ab0145532784a47ec1c5efa6b4`,
and Phase-0 binding
`03f3c4afb3c7edc167ddeadac5493cbee736042cb7781182d4fdf43b2b79166d`.

Manual constant check: the 468 `X86_FEATURE_*`/`X86_BUG_*` object-like
definitions occur in the same order in both files, and the 470 object-like
definitions after including `NCAPINTS` and `NBUGINTS` have equal names and
arithmetic expressions.  This does not resolve the following mechanism and
conditional findings.  The 949 sealed closure records cover the 472 operative
macros plus the four selected preprocessor-condition records and the scope
record; the findings below name their proposal closure keys.

## Findings

1. **P1 — `X86_BUG` no longer has macro expansion semantics.**  Linux symbol
   `X86_BUG` is the function-like preprocessor macro
   `#define X86_BUG(x) (NCAPINTS*32 + (x))` at
   `vendor/linux/arch/x86/include/asm/cpufeatures.h:524` (closure keys for its
   `selection_expression` and `status` records are
   `SC1-680242ecead2620426e910c6e49e7156be4cf1273ff16f37e07917cf4cd8bd6d`
   and `SC1-527b6d08e1c0f5d072b33fd25a7d6dc3da3affa67f3e150bfabb35c8a8da714c`,
   respectively).  The candidate instead defines `pub const fn X86_BUG(x:
   u32) -> u32` at lines 528-531.  A C macro accepts the caller's expression
   under C integer-promotion rules and expands in that caller's expression;
   the Rust function requires a `u32` argument and imposes its function/type
   boundary.  The local source contains no permitted evidence that every one
   of the 2,902 frozen header consumers supplies exactly `u32` or that this
   replacement preserves macro-context semantics.  The proposed `COMPLETE`
   closure disposition is therefore unsupported.

2. **P1 — `_ASM_X86_CPUFEATURES_H` header guard has no established Rust
   mechanism.**  Linux symbol `_ASM_X86_CPUFEATURES_H` is defined at line 3
   under the `#ifndef` at line 2 and closed at line 578.  The candidate has no
   corresponding guard or documented module/include-once equivalent.  This is
   material because the frozen header-closure record identifies 2,902 x86_64
   consumers.  The task-local proposal keys are
   `SC1-a01085e108b36323bfe0f0e936119099aa5a4eca48cb5a2048f2427ad7a09b34`
   (`selection_expression`) and
   `SC1-024815aca5f5791f82e94e47b1472eb29fa751a92a87fd64977078efd49c1b7f`
   (`status`); the conditional key is
   `SC1-eee81251678c19aad4b787cb606bb281707ad00c81ca908991fbc71c1e6d8bb1`.
   Within the permitted candidate/header/closure evidence, no source-level
   proof establishes an equivalent one-definition/include-order behavior.
   Record this as unresolved rather than marking those records `COMPLETE`.

3. **P1 — `X86_BUG_ESPFIX` is gated by an unproven Cargo feature rather than
   the frozen Kconfig predicate.**  Linux condition `#ifdef CONFIG_X86_32` at
   `cpufeatures.h:535` encloses only `X86_BUG_ESPFIX` (line 540); the frozen
   x86_64 configuration defines `CONFIG_X86_64=y` and does not define
   `CONFIG_X86_32`.  The candidate substitutes
   `#[cfg(feature = "CONFIG_X86_32")]` at line 542.  That is a different
   selection mechanism, and neither the candidate nor the permitted frozen
   closure evidence maps its Cargo feature to the frozen Kconfig value.  The
   closure keys are
   `SC1-87d77e21e754b9eb5d56b99816a965c7cc597d94cf7d3ed0099e77b4ccfca2ce`
   (`ifdef@535`) and the two `X86_BUG_ESPFIX` keys
   `SC1-5bdb0e733ba2f09a7994db46471bd6570024e3a348cced14a0859196fac79aef`,
   `SC1-b9bb2c458354308e2dc3e0d8227fa52f22b551803d94214b2bd8e6309a3ffd8c`.
   This requires an explicit frozen-configuration mapping before completion.

No unauthorized Lupos branding, value/name/order mismatch among the 470
object-like numeric definitions, shell/stub, ABI object, allocation, locking,
RCU, refcount, or lifetime mechanism appears in this header-only candidate.

**Disposition: reject the proposed semantic-closure completion pending
resolution of findings 1–3.**
