# Parity review — S000767

Reviewer: Terra (high)

Scope reviewed: `arch/x86/include/asm/xen/trace_types.h` mapped to
`src/arch/x86/include/asm/xen/trace_types.rs` for frozen x86_64.

## Result

PASS — no parity findings.

## Source comparison

- `enum xen_mc_flush_reason` is present with the exact four names and implicit
  C values: `XEN_MC_FL_NONE=0`, `XEN_MC_FL_BATCH=1`,
  `XEN_MC_FL_ARGS=2`, and `XEN_MC_FL_CALLBACK=3`.  `#[repr(C)]` retains the
  target C enum ABI used by the tracepoint field in
  `include/trace/events/xen.h`.
- `enum xen_mc_extend_args` is present with the exact three names and implicit
  C values: `XEN_MC_XE_OK=0`, `XEN_MC_XE_BAD_OP=1`, and
  `XEN_MC_XE_NO_SPACE=2`, likewise with `#[repr(C)]`.
- `xen_mc_callback_fn_t` faithfully represents `void (*)(void *)`: its
  `extern "C"` function ABI is the C ABI, its argument is a mutable `void *`
  (`*mut core::ffi::c_void`), and `Option` preserves the nullable function
  pointer representation.  The `unsafe` call requirement adds no C ABI or
  representation change and correctly leaves invocation obligations explicit.
- The candidate adds no layout-bearing fields, symbols, values, side effects,
  tests, or branding changes.  Rust has no include-guard analogue; the C
  header's only conditional is its include guard, so no selected configuration
  branch is omitted.

## Frozen selected context

`CONFIG_XEN` is unset, but `CONFIG_EVENT_TRACING=y` and the x86 Xen Makefile
builds `trace.o` for that configuration.  `arch/x86/xen/trace.c` defines trace
points through `include/trace/events/xen.h`, which directly includes this
header.  The enum types are therefore selected trace-event field types even
without Xen PV multicall code.  In Xen PV contexts, `multicalls.c` supplies the
same named values to the `xen_mc_flush_reason` and `xen_mc_extend_args` trace
events, and its callback trace event receives the declared callback-pointer
type.  No candidate conditional is required because the original header has
none beyond its include guard.

## Provenance

Candidate provenance names the exact Linux path, revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` (matching `vendor/linux.SHA`),
x86_64 architecture, and task `S000767`.
