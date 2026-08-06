# Applier resolution — S014145

Applier: `gpt-5.6-terra` (high)

## Source and frozen-task verification

- Branch is `feat/bun-like-rewrite-test`.
- `vendor/linux.SHA` and `vendor/linux` `HEAD` both resolve to
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- The frozen S014145 row maps `include/linux/irqhandler.h` to
  `src/include/linux/irqhandler.rs`, class `RUST_TRANSLATE`, architectures
  `common`. `S014140`, the mapping for `include/linux/irqdesc.h`, is still
  `TODO` and is not an S014145 dependency.
- The complete pinned source is only an include guard, a forward declaration
  of `struct irq_desc`, and `typedef void (*irq_flow_handler_t)(struct
  irq_desc *desc);`. Both independent reports were reread against that source.

## Review dispositions

1. **Parity P1 — accepted and source boundary restored.** The candidate's
   import of `crate::include::linux::irqdesc::irq_desc` was removed. The source
   now carries a local uninhabited `irq_desc` declaration solely for raw-pointer
   naming, matching the C header's intentionally incomplete declaration and
   retaining the include-cycle boundary. The alias remains
   `Option<unsafe extern "C" fn(desc: *mut irq_desc)>`: `None` preserves the
   nullable C function-pointer value, `extern "C"` preserves its call ABI,
   `*mut` preserves the mutable non-owning pointer, and `unsafe` keeps the
   caller responsible for descriptor validity and IRQ synchronization.

2. **Parity P2 — accepted and fixed.** The provenance SPDX identifier is now
   exactly `GPL-2.0`, matching `vendor/linux/include/linux/irqhandler.h:1`.

3. **Rust review — signature mechanics accepted, canonical type identity
   unresolved.** Pinned `include/linux/irq.h:16,589` includes `irqhandler.h`
   before `irqdesc.h`; pinned `include/linux/irqdesc.h:85` then stores
   `irq_flow_handler_t` in the completed `struct irq_desc`. Rust cannot declare
   a nominal type in this module and later complete that same nominal type in
   the separate `irqdesc` module. The local opaque `irq_desc` preserves this
   header's isolated forward-declaration boundary, but it is not and cannot be
   made the later `irqdesc::irq_desc` without a shared canonical opaque-type
   location or another cross-task module/ABI decision. Importing the unfinished
   `irqdesc` module would instead reverse the source boundary and introduce an
   undeclared dependency.

## Final semantic-record disposition and blocker

For both `aarch64` and `x86_64`, the `ifndef`, guard macro, and terminal
`endif` records are preprocessor-only and have no Rust runtime or ABI object.
The `irq_flow_handler_t` records resolve to a nullable C-ABI callback with a
single mutable, non-owning raw pointer to an incomplete descriptor; this header
declares no descriptor layout, ownership, allocation, locking, RCU, refcount,
cleanup, or exported storage.

The unresolved item is the required *canonical identity* of that opaque
descriptor across this header and S014140. The frozen task map supplies neither
a pre-existing shared opaque type nor an S014145 dependency that permits one.
Creating one here would make a competing nominal type; importing S014140 would
make this source depend on an unavailable task and violate the pinned C include
boundary. Exact translation therefore cannot be completed within this leased
file, so S014145 must remain `BLOCKED` until the cross-task opaque-type/ABI
representation is decided. Its Phase-0 `PENDING_REVIEW` ABI and lifetime
records cannot truthfully be marked complete before that decision.

No compiler, formatter, rust-analyzer, linker, build, test, debugger, or
runtime tool was used.
