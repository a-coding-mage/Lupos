# Resolution — S013602

Applier: `gpt-5.6-terra` (high)  
Pipeline: P01  
Disposition: **BLOCKED**

## Evidence reopened

- Pinned source: `vendor/linux/include/linux/clocksource_ids.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df` declares the eight consecutive
  enumerators `CSID_GENERIC = 0` through `CSID_MAX = 7`.
- The frozen x86_64 and aarch64 `clocksource.c` commands use LLVM 19 and do
  not include `-fshort-enums`; this establishes neither the compiler-selected
  underlying enum type nor its exact size, alignment, and signed raw-value
  ABI for this declaration.
- `rewrite/ABI.tsv` records `enum clocksource_ids` for both architectures as
  `layout=PENDING_REVIEW`, `alignment=PENDING_REVIEW`, and
  `status=PENDING_REVIEW`. `rewrite/LIFETIMES.tsv` likewise retains the task's
  semantic records as `PENDING_REVIEW`.
- Direct pinned users store and copy the enum in layout-sensitive structures
  (`struct clocksource`, `struct system_time_snapshot`,
  `struct system_counterval_t`, and `struct clock_event_device`), pass an
  `enum clocksource_ids *` through `kvm_arch_ptp_get_crosststamp`, and compare
  values directly. `kernel/time/clocksource.c:1302-1303` explicitly casts
  `cs->id` to `unsigned int`, recognizes values `>= CSID_MAX`, and repairs
  them to `CSID_GENERIC`.

## Adjudication

1. **Raw-value domain — accepted.** A Rust `#[repr(C)]` fieldless enum is not
   a valid representation for an object that the pinned code may observe with
   a value outside its declared discriminants before range-checking it. The
   present candidate therefore cannot retain the validation-and-repair path's
   source behavior.
2. **Scalar copy and equality — accepted.** The candidate provides neither
   trivial copy nor integer-value equality, while the pinned callers assign,
   read, and compare this scalar directly. A corrected raw scalar wrapper
   would need these semantics without `Drop`, allocation, or panic behavior.
3. **ABI/layout — accepted and blocking.** The required exact underlying
   representation for both frozen targets is not established by the pinned
   declaration, the frozen command records, or the current authoritative ABI
   and lifetime manifests. Choosing `i32`, `u32`, or another wrapper backing
   type would be an unsupported ABI assumption and could alter embedded-field
   offsets and the pointer-facing contract.

## Result

No candidate source change was made: an apparently corrected integer wrapper
would still guess the unresolved ABI. The task's enum ABI, raw-value behavior,
copy/equality semantics, layout/alignment, and embedded/pointer lifetime
contract remain unresolved `PENDING_REVIEW` records. Per the source-parity
rule, this task must remain blocked until frozen authoritative ABI evidence
establishes the exact representation for x86_64 and aarch64.
