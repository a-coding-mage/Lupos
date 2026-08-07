# Application resolution — S000496

Pinned source reopened: `vendor/linux/arch/x86/include/asm/cpufeatures.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, complete lines 1–578.
The frozen task is x86_64-only (`rewrite/SCOPE.tsv:497`).  Its frozen
configuration has `CONFIG_64BIT=y` and `CONFIG_X86_64=y`
(`rewrite/configs/x86_64/frozen.config:313-314`) and has no
`CONFIG_X86_32`; the recorded x86 compile command is `--target=x86_64-linux-gnu`
with `-m64` (`rewrite/FILE_MAP.tsv:19823`).  No compiler, formatter, linker,
test, diagnostic, or historical Rust source was used.

## Parity review finding 1 — BLOCKED

Disposition: accepted.  The candidate changes all object-like macro
expressions to `usize` constants and changes `X86_BUG(x)` into a
`usize -> usize` function.  Upstream lines 8-9, 21-519, and 524-577 instead
provide unsuffixed C integer-token expressions.  In particular,
`X86_BUG(x)` expands to `(NCAPINTS*32 + (x))` and therefore applies C usual
arithmetic conversions to its single evaluated argument at the caller's
expression type; it is not a pointer-width unsigned API.

The requested Rust source representation cannot be established exactly in the
allowed project context.  A typed Rust constant would fix the value type before
the caller, while the C macro is token substitution.  A Rust `const fn` fixes
the parameter/result type; a `macro_rules!` expansion still cannot reproduce
C integer promotions and usual arithmetic conversions for every permitted
integer argument.  The selected consumer header demonstrates distinct C
conversion boundaries: `arch/x86/include/asm/cpufeature.h:51-79` passes the
indices to `set_bit` and to `unsigned int` parameters, while lines 99-116 pass
them to `_static_cpu_has(u16)` and form `1 << (bit & 7)` masks.  The selected
boot consumer forms 32-bit `1 << X86_FEATURE_*` masks at
`arch/x86/boot/cpucheck.c:130-145`.  No approved Rust integer/macro mapping in
`SYMBOLS.tsv`, `ABI.tsv`, or `LIFETIMES.tsv` closes these different contexts;
the S000496 semantic rows remain `PENDING_REVIEW`.

The precise blocker is the required mapping of `NCAPINTS`, `NBUGINTS`, every
`X86_FEATURE_*`, every `X86_BUG_*`, and function-like `X86_BUG(x)` without
changing caller-context C integer conversion, shift/mask width, signedness, or
overflow behavior.  Replacing `usize` with another fixed Rust scalar, or
retaining the function, would guess that contract.

## Rust review finding 1 — BLOCKED

Disposition: accepted for the same unresolved source-semantic reason as the
parity finding above.  The upstream `X86_BUG(x)` at line 524 has an `int`
literal left operand and a parenthesized one-time argument use.  Its frozen
internal invocations use integer literals, but the header macro itself remains
an operative selected macro (`rewrite/SYMBOLS.tsv:23793`) exposed through the
selected header closure (2,902 consumers in `rewrite/metadata/header_closure.tsv:7893`).
No exact Rust equivalent of its C caller-context arithmetic has been supplied
by the frozen records.  The candidate's `usize` function is therefore not
accepted.

## Rust review finding 2 — RESOLVED

Disposition: corrected in `src/arch/x86/include/asm/cpufeatures.rs`.  Upstream
guards `X86_BUG_ESPFIX` only with `#ifdef CONFIG_X86_32` at lines 535-541.
For the sole frozen x86_64 configuration that predicate is inactive.  The
candidate's unrelated `#[cfg(target_arch = "x86")]` and the out-of-scope
32-bit constant were removed.  The active x86_64 translation now has no
`X86_BUG_ESPFIX` item, matching the frozen Kconfig result rather than the host
or Rust target architecture.

## Outcome

The configuration finding is resolved, but the integer/macro contract is not
establishable from the permitted source and frozen records.  This task must be
`BLOCKED`; it is not eligible for `DONE`.
