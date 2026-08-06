# Parity review — S013602

Reviewer: parity_reviewer (`gpt-5.6-terra`, high)  
Pipeline: P01  
Scope: `include/linux/clocksource_ids.h` → `src/include/linux/clocksource_ids.rs`  
Method: source-only comparison; no compiler, formatter, analyzer, build, test, or runtime tool was used.

## Evidence reviewed

- `vendor/linux.SHA` resolves to `425f94c2954b1fe80ebdbf9b29854e89750355df`, the checked-out pinned Linux revision.
- The queue row is `REVIEWING` for `S013602` under `P01`; its frozen source/destination pair and `common` architecture coverage match the candidate provenance.
- `rewrite/SCOPE.tsv` selects this header through the frozen aarch64 and x86_64 header closures. `rewrite/SYMBOLS.tsv` records only the header guard and `enum clocksource_ids`, with no configuration-selected branches inside the header.
- Upstream `include/linux/clocksource_ids.h:6-15` declares the one enum and all eight consecutive values: `CSID_GENERIC = 0`, `CSID_ARM_ARCH_COUNTER = 1`, `CSID_S390_TOD = 2`, `CSID_X86_TSC_EARLY = 3`, `CSID_X86_TSC = 4`, `CSID_X86_KVM_CLK = 5`, `CSID_X86_ART = 6`, and `CSID_MAX = 7`.
- The candidate retains the exact type/variant names, ordering, numeric sequence, `common` provenance, and uses `#[repr(C)]`. There are no functions, statics, linkage directives, or configuration branches to compare. The C include guard has the normal Rust module-equivalent single definition role.

## Findings

1. **High — the closed Rust enum cannot preserve the C enum's validated raw-value domain.**

   `src/include/linux/clocksource_ids.rs:10-20` represents the type as a Rust `#[repr(C)]` enum, which has only the eight valid discriminants. The Linux type is a C scalar field in public structures (for example, `include/linux/clocksource.h:44-47`). Linux explicitly accepts an integer-valued `cs->id`, tests it against the sentinel, and normalizes an out-of-range value: `kernel/time/clocksource.c:1302-1303` casts `cs->id` to `unsigned int`, checks `>= CSID_MAX`, then assigns `CSID_GENERIC`. A Rust enum may not safely contain a non-discriminant bit pattern, so the candidate makes precisely the invalid-yet-checked state unrepresentable/undefined before this Linux validation can run. The applier must use a representation that preserves the pinned C enum ABI and all raw values until validation (with the exact underlying C enum width/alignment resolved for both frozen targets), while retaining the named constants.

2. **High — the candidate loses C scalar copy and comparison semantics.**

   C enum objects are freely copied and compared as scalar values. The candidate declares no `Copy`, `Clone`, `PartialEq`, or `Eq` implementation. Consequently, a stored Rust `clocksource_ids` value moves on ordinary use and cannot participate in the direct equality expressions that upstream uses, including `kernel/time/timekeeping.c:1567-1568` and `drivers/ptp/ptp_vmclock.c:241`. This is not an optional convenience trait: downstream translations must preserve the C assignments, field reads, and equality checks. The applier must provide the equivalent scalar operations as part of the corrected representation.

## Required resolution

Do not close this task as `DONE` with the present declaration. Resolve both findings from pinned source and frozen ABI evidence, update the candidate, and have the final applier record the resulting ABI/lifetime decisions for both architectures.
