# Resolution — S000749 / P01 / attempt 1

## Outcome

**BLOCKED.**  I reopened the complete pinned
`vendor/linux/arch/x86/include/asm/vermagic.h`, its direct x86_64 configuration
selection, `include/linux/vermagic.h`, `kernel/module/main.c`, and
`scripts/module-common.c`, together with the frozen task records and both
independent review attestations.  The sealed candidate was not modified.

The frozen x86_64 configuration selects `CONFIG_X86_64=y` and not
`CONFIG_X86_32`, so upstream line 49 selects the empty string-literal
replacement list.  That selection alone is insufficient to establish the
required Rust representation and consumer boundary.

## Findings and dispositions

### Parity P1 / Rust RUST-S000749-1 — `MODULE_ARCH_VERMAGIC` token and byte-array contract

**Accepted; unresolved.**  `MODULE_ARCH_VERMAGIC` is an object-like C macro
whose selected replacement list is the literal token `""`, not a C object or
pointer.  `include/linux/vermagic.h:41-46` composes that replacement list into
`VERMAGIC_STRING` by adjacent C string-literal expansion.  In the pinned
consumer, `kernel/module/main.c:1105` uses the resulting token sequence to
initialize `static const char vermagic[]`; `kernel/module/main.c:2635-2649`
then uses that static C string when checking module version magic.
`scripts/module-common.c:20-21` also passes the composed token sequence to
`MODULE_INFO(vermagic, ...)`.

The candidate's `&str` is a Rust slice value, not a caller-expanded literal
token sequence or a static NUL-terminated C byte-array initializer.  The
frozen source and ABI records do not define a Rust macro/token mechanism,
translated-consumer contract, ownership rule, or FFI boundary that would
preserve both uses without inventing new behavior.  The requested exact
mapping therefore cannot be established in this per-header task.

### Parity P2 — include guard and deliberate absence of `MODULE_PROC_FAMILY`

**Accepted; unresolved.**  `vermagic.h:3-4,52` supplies an inclusion guard,
and its selected `CONFIG_X86_64` arm at lines 6-7 deliberately defines no
`MODULE_PROC_FAMILY`.  Those facts govern C preprocessing state before the
later `MODULE_ARCH_VERMAGIC` expansion.  Rust module loading does not provide
a textual C preprocessor environment, and the frozen records supply no
source-proven bridge that represents the guard or the defined-versus-undefined
macro state at the translated header/consumer boundary.  Mere prose and a
Rust constant do not preserve that behavior.

## Blocking record

The semantic records cited by the reviewers remain unresolved:

- `SC1-71a785d3a41a120d42da2fd804bbe79a0e2e3cdb5f521538bcd020864adaa019`
- `SC1-b78793537e71a7343e04d442def44b8878786bb0ceb039fd123cda316e859098`
- `SC1-065d47e15e6a4b064b134ce412240c7de0c1cc9ddbe6e14ec4e813353ce2563b`
- `SC1-446091066727aa422735771735e8a31ac1e6dad92b5a238f10e65e3ece52131f`

No source-only evidence permits closing them without a new frozen,
consumer-level macro/FFI representation.  Phase 1 therefore requires this
task to remain blocked rather than accepting an invented `&str` substitute.
