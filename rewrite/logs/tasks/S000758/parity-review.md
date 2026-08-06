# Parity review — S000758 (slot 1)

Verdict: **REJECT — two findings require applier disposition.**

Reviewed candidate: `src/arch/x86/include/asm/vmxfeatures.rs` against pinned
`vendor/linux/arch/x86/include/asm/vmxfeatures.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Inventory checks that passed

- The candidate provenance names the exact Linux source, frozen x86_64-only
  architecture, task `S000758`, and the pinned revision, which matches
  `vendor/linux.SHA`.
- The source has exactly 64 `VMX_FEATURE_*` object-like macros; the candidate
  exports exactly 64 same-named public constants. Name-set comparison and
  evaluation of every `word * 32 + bit` expression found no difference.
- `NVMXINTS` has value 5 in both source and candidate. The header has no
  configuration conditional other than its include guard. The frozen x86_64
  configuration enables `CONFIG_X86_VMX_FEATURE_NAMES=y`.

## Findings

### P1 — all index macros were changed from signed C `int` expressions to `u32`

`NVMXINTS` and every `VMX_FEATURE_*` definition in the source consists only of
unsuffixed decimal integer literals, so their C expression type is signed
`int` on the pinned x86_64 target. The candidate declares all 65 values as
`u32` (`vmxfeatures.rs:9` and `:12-81`). This is a semantic type change rather
than a mere Rust spelling change: the source uses `NVMXINTS` as an array bound,
in an `int` loop bound, and in `BUILD_BUG_ON(NVMXINTS !=
NR_VMX_FEATURE_WORDS)` (`processor.h:153`, `proc.c:13,112`, and
`feat_ctl.c:30`). Feature indices are also intentionally subjected to C usual
arithmetic conversions by `VMX_F(x) BIT(VMX_FEATURE_##x & 0x1f)`
(`feat_ctl.c:24`).

Required resolution: preserve the source's signed integer macro semantics in
the Rust mapping (with explicit, use-site conversions where Rust requires an
array index or unsigned shift), or provide pinned-source evidence for a
different frozen ABI/type contract. Do not leave a blanket `u32` declaration
without such evidence.

### P2 — candidate drops operative quoted-name metadata used to form VMX flag output

The header explicitly specifies that a feature macro comment beginning with a
quoted string supplies that feature's `/proc/cpuinfo` name; a non-quoted
comment suppresses its display (`vmxfeatures.h:10-14`). The frozen
configuration enables this behavior. `arch/x86/kernel/cpu/mkcapflags.sh:65-70`
includes this header and parses those quoted comments into the
`x86_vmx_flags[NVMXINTS*32]` table. `proc.c:109-115` emits that table as
`vmx flags` in `/proc/cpuinfo`.

The candidate retains none of the 29 quoted strings (for example `"vnmi"`,
`"ept"`, `"vpid"`, and `"ipi_virt"`), nor an equivalent name-at-index
mapping. Therefore the Rust translation itself cannot reproduce this selected,
user-visible metadata contract. The original generated `capflags.c` is
classified BUILD_METADATA, but that classification does not by itself record
how this Rust header exports or preserves the required metadata.

Required resolution: retain an exact index-to-name representation in the Rust
source or explicitly establish, with frozen build/ABI evidence, that the
original generated C object is the authoritative provider of
`x86_vmx_flags` and remains linked against the Rust core. In either case,
preserve the distinction between quoted and unquoted feature comments.

No source, manifest, or queue edits were made by this reviewer. No build,
format, test, runtime, or compiler command was run.
