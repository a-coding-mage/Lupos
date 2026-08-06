# Parity review — S012594

Reviewer: parity_reviewer (`gpt-5.6-terra`, high)

## Review basis

- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df` (matches
  `vendor/linux.SHA` and `vendor/linux` HEAD).
- Queue row: `REVIEWING`, pipeline `P01`, source
  `include/asm-generic/trace_clock.h`, destination
  `src/include/asm-generic/trace_clock.rs`, architecture `aarch64`.
- Scope record S012594: `RUST_TRANSLATE`, selected through the AArch64 header
  closure (5,089 consumers). The frozen AArch64 config has
  `CONFIG_TRACE_CLOCK=y`; no `arch/arm64/include/asm/trace_clock.h` override
  exists.
- Source inspected: the complete pinned header; its direct consumer
  `vendor/linux/kernel/trace/trace.c`; and
  `vendor/linux/include/linux/trace_clock.h`.

## Finding P1 — operative `ARCH_TRACE_CLOCKS` macro has no candidate mapping

`vendor/linux/include/asm-generic/trace_clock.h:13-15` conditionally defines
`ARCH_TRACE_CLOCKS` as an empty replacement list when an architecture has not
already supplied it. This is operative selected behavior, not merely a comment
or include guard: `vendor/linux/kernel/trace/trace.c:1080` expands it as an
element position in the `trace_clocks[]` initializer. For the selected AArch64
configuration, the generic header is used and its expansion must add no
entries.

`src/include/asm-generic/trace_clock.rs:7-10` only describes that fact in a
doc comment. It defines neither an equivalent compile-time item/macro nor an
explicit mapping for the translated trace-clock initializer. Consequently, the
selected operative macro listed in `rewrite/SYMBOLS.tsv` has no source
representation, and downstream translated code cannot obtain the source
header's default/no-entry contribution from this module.

Required resolution: supply an exact Rust-level representation of the default
empty trace-clock contribution, or make the downstream initializer's
architecture-conditioned empty contribution explicit and record that
source-level mapping. The resolution must preserve the C behavior that an
architecture-specific definition takes precedence and that the generic AArch64
path contributes zero initializer entries.

## Other checks

The candidate's immutable provenance lines match the task source, pinned
revision, architecture, and task ID. It contains no test configuration,
placeholder macro, or unauthorized branding. No ABI/layout/linkage behavior is
present in this header beyond its compile-time macro contract.

No compiler, formatter, build, test, debugger, or rust-analyzer diagnostic was
run or used in this review.
