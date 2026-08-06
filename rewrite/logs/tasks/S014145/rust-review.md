# Rust review — S014145

Reviewer: `gpt-5.6-terra` (`high`), slot 2 (Rust semantics)  
Scope: `src/include/linux/irqhandler.rs` against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, source review only. No compiler,
formatter, analyzer, build, test, or runtime tool was used.

## Result

No Rust-semantics finding requiring a source change.

## Checked ABI, nullability, and opaque-pointer semantics

- The complete upstream header declares only an incomplete `struct irq_desc`
  and `typedef void (*irq_flow_handler_t)(struct irq_desc *desc);`
  (`vendor/linux/include/linux/irqhandler.h:9-12`).  The candidate imports the
  canonical descriptor name and represents its sole argument as `*mut
  irq_desc` (`src/include/linux/irqhandler.rs:12,20`).  A raw mutable pointer
  retains C's nullable, non-owning, non-exclusive pointer contract; it neither
  asserts a Rust borrow nor supplies a destructor/lifetime.
- `Option<unsafe extern "C" fn(...)>` at candidate line 20 is the appropriate
  nullable representation of the C function-pointer typedef: `None` is the
  null function pointer and `Some` carries the C ABI callback.  This is
  material: upstream accepts a null handler and substitutes `handle_bad_irq`
  (`vendor/linux/kernel/irq/chip.c:947-953`), while a configured handler is
  subsequently stored and invoked (`vendor/linux/kernel/irq/chip.c:987-990`; 
  `vendor/linux/include/linux/irqdesc.h:186-189`).
- The `unsafe` callback type is justified rather than an ABI change.  Invocation
  requires the caller to uphold the kernel's raw-descriptor validity and IRQ
  synchronization contracts, and the alias does not incorrectly promote that
  pointer to `&mut irq_desc`.  The upstream descriptor may be freed through
  RCU after its reference is released (`vendor/linux/kernel/irq/internals.h:109-119`; 
  `vendor/linux/kernel/irq/irqdesc.c:466-474`), so adding a Rust lifetime or
  ownership wrapper here would be unsound.
- The import deliberately uses the canonical descriptor type to be completed
  by the separately inventoried `include/linux/irqdesc.h` task S014140; this
  preserves the original header's incomplete-type role without inventing a
  competing opaque layout.  S014140 is presently a frozen-queue `TODO` task,
  so this review makes no claim that the unfinished whole tree compiles.

## Frozen-record disposition for the applier

`rewrite/SYMBOLS.tsv`, `rewrite/ABI.tsv`, and `rewrite/LIFETIMES.tsv` each
retain the S014145 rows as `PENDING_REVIEW`.  Source evidence above resolves
their task-local substance as: C-ABI nullable function pointer; argument is a
non-owning raw pointer to an incomplete `irq_desc`; no layout-bearing object or
owned lifetime is declared by this header.  The applier must record those final
dispositions before `DONE` as required by the workflow.
