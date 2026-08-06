# S016070 applier resolution

Applier reopened the pinned `include/uapi/linux/bpf_common.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 and AArch64
configuration records, the task symbol inventory, and the selected consumer
evidence in `include/uapi/linux/bpf.h`, `include/uapi/linux/filter.h`,
`include/linux/filter.h`, `kernel/bpf/*.c`, and `net/core/filter.c`.

## Parity P1 / Rust finding 1 — resolved

`BPF_CLASS`, `BPF_SIZE`, `BPF_MODE`, `BPF_OP`, and `BPF_SRC` are now exported
`macro_rules!` expression macros. Each expands its operand once, casts the
selected BPF instruction-code operand to `core::ffi::c_int`, and applies the
same positive upstream mask (`0x07`, `0x18`, `0xe0`, `0xf0`, or `0x08`). The
expansion is therefore available in Rust constant and initializer contexts;
it no longer crosses a function or trait-method call boundary.

The selected operand facts are source-backed: `struct bpf_insn.code` is
`__u8` in `include/uapi/linux/bpf.h:80-86`, `struct sock_filter.code` is
`__u16` in `include/uapi/linux/filter.h:24-29`, and the construction macros
in `include/linux/filter.h:106-143` compose these masks with `int` BPF
constants. On both frozen LP64 targets, those unsigned narrow fields promote
to 32-bit `int` before the upstream mask operation. The new macro expansions
preserve that selected conversion, their `int` result, parentheses, and
single evaluation.

## Rust finding 2 — resolved

The sealed `BpfCodeMaskInput` trait and every associated helper method were
removed. There is no longer a closed Rust-only operand set, runtime dispatch,
or helper symbol. The public operation is a macro expansion, as in the pinned
header; the frozen selected BPF field categories above map directly to the C
promotion result rather than being admitted through a trait.

## Parity P2 / Rust finding 3 — resolved

`BPF_MAXINSNS` now preserves the source default/override structure. With no
`bpf_maxinsns_override` feature, the header provides the pinned default
`c_int` value `4096`. With that compile-time feature, it re-exports the
inclusion-time replacement `crate::BPF_MAXINSNS_OVERRIDE` under the upstream
public name instead of defining the default. This is the Rust configuration
equivalent of `#ifndef BPF_MAXINSNS`. The frozen source/configuration evidence
contains no prior `BPF_MAXINSNS` definition or Kbuild `-D` override; its
selected behavior is consequently the source default on both architectures.

## Inventory and semantic records

All 92 S016070 `SYMBOLS.tsv` records (46 for each architecture) were closed
against the pinned declaration and frozen configuration evidence. The five
function-like macros are recorded as exported `c_int` expression macros; all
object-like values are recorded as `c_int` literals; the header/default guards
are recorded with their Rust module and feature-gated mappings. This task has
no `ABI.tsv` or `LIFETIMES.tsv` rows: it declares no data layout, linkage,
ownership, allocation, locking, RCU, refcount, or lifetime-bearing entity, so
those two record families are not applicable.

No compile, formatter, test, runtime, or benchmark command was run.
