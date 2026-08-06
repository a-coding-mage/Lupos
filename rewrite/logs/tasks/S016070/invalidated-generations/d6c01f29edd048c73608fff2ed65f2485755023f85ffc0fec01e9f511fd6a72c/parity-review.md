# Parity review — S016070 — slot 1

Reviewed candidate: `src/include/uapi/linux/bpf_common.rs` against pinned
`vendor/linux/include/uapi/linux/bpf_common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Result: reject pending correction

All 36 source-defined BPF value macros are represented with their upstream
`int` literal values: instruction classes (lines 7–14), size fields (18–20),
mode fields (23–28), ALU/JMP operation fields (32–48), source fields (50–51),
and `BPF_MAXINSNS` (54). The candidate correctly does **not** define the
commented-out `BPF_DW`. Its SPDX identifier, Linux source path, revision,
architecture, and task provenance match the task and pinned source. No
branding delta was found.

### P1 — Function/trait replacement loses macro expansion and integer-expression contexts

Upstream defines each of `BPF_CLASS`, `BPF_SIZE`, `BPF_MODE`, `BPF_OP`, and
`BPF_SRC` as a function-like preprocessor macro expanding directly to
`((code) & <int mask>)` (bpf_common.h:6,17,22,31,49). The candidate instead
exports five ordinary generic functions (bpf_common.rs:106–108, 121–123,
131–133, 144–146, 167–169) mediated by the public `BpfCodeMaskInput` trait
(lines 7–102).

This is not an equivalent replacement for selected consumers:

- An upstream expansion participates in the surrounding expression and is a
  constant expression whenever its operand is one. An ordinary, non-`const`
  function call cannot do so.
- Function calls impose a separate typed call boundary and cannot reproduce
  the macro's ordinary C expression conversion/assignment context. In
  particular, `struct bpf_insn.code` is `__u8` (include/uapi/linux/bpf.h:80–86),
  while `include/linux/filter.h` builds instruction initializers using the
  macros directly in `.code = BPF_* | BPF_OP(OP) | BPF_*` at lines 106–143,
  291–430. The upstream expression is computed as `int` and then assigned to
  the `__u8` field; the candidate's generic return type and function call are
  not the same expression form and make faithful translations of those
  initializers dependent on extra, non-source behavior.
- The public trait only admits a hand-selected set of Rust primitive types.
  Upstream's macro admits every integer/enum expression accepted by C bitwise
  `&`, with the target compiler's usual arithmetic conversions. It has no
  trait bound and introduces no public callable symbols or helper methods.
  The sealed trait also prevents a translated C enum or other integer wrapper
  from expressing the upstream operation without altering this header.

Required resolution: represent the five operations with an expansion mechanism
that preserves the caller's expression/constant context and establishes exact
conversion behavior at each selected operand type, rather than exporting a
runtime generic trait/function API. Re-review the affected source after that
semantic mapping is supplied.

### P2 — `BPF_MAXINSNS` loses its guarded definition contract

The pinned UAPI header intentionally uses `#ifndef BPF_MAXINSNS` before
defining `BPF_MAXINSNS` as `4096` (bpf_common.h:53–55; recorded for both
architectures in `rewrite/SYMBOLS.tsv`). The candidate unconditionally exports
`pub const BPF_MAXINSNS: i32 = 4096` (bpf_common.rs:174), so a prior approved
definition cannot remain effective. No alternate configuration/override
mechanism is documented in the candidate or task records.

Required resolution: preserve the frozen UAPI override/selection contract, or
record and implement the exact Rust-side equivalent supported by the selected
build/module configuration.

## Audit coverage

`rewrite/SYMBOLS.tsv` contains 46 selected records for S016070 under each of
`x86_64` and `aarch64`, covering both include/conditional records and every
operative macro. The source is 57 lines and all source definitions, masks,
and values were compared. `rewrite/SCOPE.tsv` identifies this as common,
`RUST_TRANSLATE`, with metadata header consumers for both frozen architectures;
current `src/` has no other materialized BPF macro caller yet, so the pinned
Linux consumers above are the operative parity evidence.
