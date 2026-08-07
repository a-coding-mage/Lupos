# Rust source review — S000496, slot 2, attempt 1

Verdict: **findings; not source-acceptable without resolution.**  This was a
manual source review only.  No compiler, formatter, test, rust-analyzer,
runtime tool, or historical Rust source was used.

## Scope and snapshot

- Queue row is `REVIEWING`, pipeline `P02`, attempt `1`, for the frozen
  x86_64 source `arch/x86/include/asm/cpufeatures.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The proposal correctly binds its candidate snapshot: `candidate.diff` is
  `fe7d25e0322ebf96e3cfde98a33f30d38e1fd0f64dd4944987fe7ad8a260d433`.
  Proposal and seal are respectively
  `524981d2602656a5dd7b370849adc3c1c7ab0e9c7131cfba3403a579a09a88ea`
  and `761a592097f9ecebdfa70b5cc6005a274e32f5d750f772e72a0cf2eb279cce24`.
- I audited all 471 numeric macro expansions (the 472nd selected item is the
  include-guard macro).  Their identifiers and arithmetic tokens agree with
  the upstream definitions.  The maximum resulting index is below 32-bit
  range, so no individual multiplication/addition overflows.  There are no
  shifts, raw pointers, `unsafe`, FFI declarations, allocation, panics,
  callbacks, refcounts, interior mutability, `Drop`, `Send`/`Sync`, layout, or
  endian operations in this file.  Those absences do not cure the following
  source-level mechanism changes.

## Findings

### RUST-001 — high — every macro expression has been given a fixed `u32` type

Upstream defines `NCAPINTS`, `NBUGINTS`, and every `X86_FEATURE_*`/`X86_BUG_*`
as C preprocessor integer expressions (for example, `NCAPINTS` at
`vendor/linux/arch/x86/include/asm/cpufeatures.h:8` and the feature table
starting at line 21).  Their C type and conversions are selected by the use
site under the C integer-promotion rules.  The candidate instead fixes every
item to `u32` (starting at
`src/arch/x86/include/asm/cpufeatures.rs:12`).

This changes the public expression type and hence signed comparison,
subtraction, bit-operation, indexing, and mixed-width conversion behavior for
every consumer; it is not a representation-neutral transcription of a C macro.
Although all presently written literals are non-negative and in range, that
does not restore contextual C conversion behavior.  Resolve with a
source-grounded macro/expression translation strategy or demonstrate the
frozen consumer contract for the selected macro API; do not accept a blanket
`u32` substitution.

Closure keys: `SC1-e265a9c2d7905e60b45a0b756555fa64f4cbedb82c803b361e991a902248ed88`
and `SC1-58c80dca3c2009308beae405c9225dabf0c66ee6d3dfe804dab69209073c5b9f`
(`NCAPINTS`), together with the proposal's corresponding per-macro
selection/status records for the audited table.

### RUST-002 — high — `X86_BUG` no longer has its upstream macro contract

Upstream line 524 defines `X86_BUG(x)` as the call-site macro expression
`(NCAPINTS*32 + (x))`.  Candidate lines 528–531 replace it with a public
`const fn X86_BUG(x: u32) -> u32`.  `#[inline]` is not an equivalence proof:
the function imposes a `u32` argument/result at the call site, is an ordinary
Rust item that can be named as a function, and cannot retain the macro's
contextual C promotions.  This also propagates RUST-001's type change to all
`X86_BUG_*` constants and any selected direct use of `X86_BUG`.

Closure keys: `SC1-680242ecead2620426e910c6e49e7156be4cf1273ff16f37e07917cf4cd8bd6d`
and `SC1-527b6d08e1c0f5d072b33fd25a7d6dc3da3affa67f3e150bfabb35c8a8da714c`
(`X86_BUG`, upstream line 524).

### RUST-003 — medium — the x86_32 conditional has no established frozen-Kconfig binding

The pinned x86_64 configuration sets `CONFIG_X86_64=y` and does not define
`CONFIG_X86_32`; consequently upstream's `#ifdef CONFIG_X86_32` at lines
535–541 excludes `X86_BUG_ESPFIX`.  Candidate line 542 instead consults a
Cargo feature named `CONFIG_X86_32`.  The reviewed source and frozen manifest
provide no mapping that makes that Rust feature exactly track the pinned Linux
Kconfig predicate.  A Cargo feature may therefore expose the symbol in the
frozen x86_64 build or hide it under a future selected configuration for an
unrelated build-system reason.  Preserve the frozen predicate mechanically or
make the x86_64 selection explicit from authoritative configuration evidence.

Closure key: `SC1-87d77e21e754b9eb5d56b99816a965c7cc597d94cf7d3ed0099e77b4ccfca2ce`
(`ifdef@535`).

### RUST-004 — medium — the selected include-guard macro has been closed without an evidenced Rust equivalent

`_ASM_X86_CPUFEATURES_H` is an operative selected macro at upstream lines 2–3.
Candidate has no equivalent guard or documented module-level mechanism.  Rust
modules can prevent duplicate item definitions, but that is not the same
source-level contract as the C include guard and no permitted evidence ties
the Rust inclusion/module topology to the header's repeat-inclusion behavior.
The semantic closure must not mark this record COMPLETE until that mechanism
is established from the frozen translation context.

Closure keys: `SC1-eee81251678c19aad4b787cb606bb281707ad00c81ca908991fbc71c1e6d8bb1`,
`SC1-a01085e108b36323bfe0f0e936119099aa5a4eca48cb5a2048f2427ad7a09b34`,
and `SC1-024815aca5f5791f82e94e47b1472eb29fa751a92a87fd64977078efd49c1b7f`.

