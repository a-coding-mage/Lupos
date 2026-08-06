# Rust review — S016014

Role: `rust_reviewer`  
Pipeline: `P02`  
Model: `gpt-5.6-terra`, high reasoning effort  
Scope reviewed: `vendor/linux/include/uapi/asm-generic/param.h` against
`src/include/uapi/asm-generic/param.rs`, plus its direct include hierarchy and
the frozen x86_64/aarch64 records. Source inspection only; no compiler,
formatter, linker, test, or rust-analyzer diagnostics were used.

## Result: changes required

### R1 — High: unconditional constants lose four selected macro-override contracts

The source header deliberately supplies defaults only:

- lines 5–7: `__USER_HZ` only when not already defined;
- lines 9–11: `HZ` only when not already defined;
- lines 13–15: `EXEC_PAGESIZE` only when not already defined;
- lines 17–19: `NOGROUP` only when not already defined.

The candidate instead defines all four names unconditionally as public module
constants (lines 8, 11, 14, and 17). A Rust `const` cannot be preprocessor-
suppressed or replaced by an earlier architecture/header definition. This is
not merely an implementation detail: `arch/arm64/include/uapi/asm/param.h:20–22`
defines `EXEC_PAGESIZE` as `65536` *before* including the generic header, so
the selected arm64 UAPI result is 65536, whereas the candidate permanently
publishes 4096. `rewrite/SCOPE.tsv` records that arm64 header as task S000217,
dependent on S016014.

The translation must preserve the source-level override/composition mechanism
in the architecture-selected Rust namespace (or record an exact equivalent)
rather than expose generic defaults as final, unconditional common constants.

### R2 — High: `HZ` cannot serve the selected kernel include hierarchy as `100`

`include/asm-generic/param.h:5–10` first includes this UAPI header, then
explicitly `#undef HZ` and defines `HZ` to `CONFIG_HZ`; this is the internal
kernel timer frequency, not the UAPI default. The frozen configurations select
`CONFIG_HZ=1000` for x86_64 (`rewrite/configs/x86_64/frozen.config:469–470`)
and `CONFIG_HZ=250` for aarch64
(`rewrite/configs/aarch64/frozen.config:470–473`). Candidate line 11 publishes
one common `HZ: i32 = __USER_HZ` (100), with no architecture-selected internal
wrapper or mechanism by which the later `#undef` can be modeled. Its public
name must not become the value consumed by the internal `asm-generic/param`
translation; the eventual hierarchy must keep the UAPI default distinct from
the architecture/configuration-specific kernel `HZ`.

### R3 — Medium: the candidate narrows contextual C macro semantics to `i32`

All five source values are replacement tokens, not declared ABI objects or
typed C constants. At an isolated expression each unsuffixed decimal literal
and `(-1)` has C `int` type on the pinned targets, but macro expansion permits
the caller's ordinary C conversions. For example, `NOGROUP` can participate in
an unsigned expression as the converted all-ones value, while a Rust
`pub const NOGROUP: i32` requires callers to opt into a conversion and changes
which mixed-type expressions are accepted. `HZ` also expands to another macro
and therefore inherits that macro's selected value and contextual conversion.

An exact Rust representation needs documented, call-site-appropriate fixed
width/signedness handling rather than treating this generic UAPI source as a
single globally typed `i32` API. The review does not identify a `repr(C)` item
in this header: it declares no struct, union, enum, function, or linkable data
object. The issue is macro/source namespace and expression semantics, not a
missing representation attribute.

## Required manifest closure before `DONE`

`rewrite/SYMBOLS.tsv:320995–321026` retains every selected conditional and
operative macro for both architectures as `PENDING_REVIEW` (including all of
`__USER_HZ`, `HZ`, `EXEC_PAGESIZE`, `NOGROUP`, and `MAXHOSTNAMELEN`). No
S016014-specific row is present in `rewrite/ABI.tsv` or
`rewrite/LIFETIMES.tsv`. The applier must close these semantic records with the
upstream evidence above, explicitly recording that this macro-only header has
no layout/linkage/lifetime object where that is the conclusion, and resolve the
override and target-namespace design. No allowlisted branding difference was
observed. Provenance fields in candidate lines 1–5 match the source path,
common architecture scope, task ID, and `vendor/linux.SHA` revision.

