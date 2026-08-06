# Rust semantics review — S013602

Reviewed manually and source-only: `vendor/linux/include/linux/clocksource_ids.h` and
`src/include/linux/clocksource_ids.rs`, with the frozen S013602 symbol/ABI/lifetime
records and direct pinned users of the type. The candidate preserves the eight
enumerator numeric values (0 through 7), but is not ready for acceptance.

## Findings

1. **High — the Rust enum rejects representations that the Linux implementation
   explicitly handles.**

   `enum clocksource_ids` is stored in mutable shared C structures, including
   `struct clocksource::id` (`include/linux/clocksource.h:131-135`),
   `struct system_time_snapshot::{cs_id,hw_csid}`
   (`include/linux/timekeeping.h:294-302`), and
   `struct clock_event_device::cs_id` (`include/linux/clockchips.h:105-117`).
   Linux deliberately checks and repairs a value outside the defined range:
   `if (WARN_ON_ONCE((unsigned int)cs->id >= CSID_MAX)) cs->id = CSID_GENERIC;`
   (`kernel/time/clocksource.c:1302-1303`).

   A Rust fieldless enum has a validity invariant restricting its in-memory value
   to its declared discriminants. `#[repr(C)]` selects a C-style layout but does
   not make arbitrary integer bit patterns valid Rust enum values. Consequently,
   loading a malformed or concurrently supplied C representation through this
   type is not equivalent to Linux's range-check-and-repair path and can introduce
   undefined behavior before that path can run. The final representation must
   retain the frozen C enum's exact ABI while permitting the raw integer domain
   Linux validates (for example, an explicitly ABI-verified transparent integer
   wrapper with named constants), rather than using a validity-restricted Rust
   enum.

2. **High — the candidate does not have C's copy and comparison semantics.**

   C enum values are ordinary scalar values. Pinned code copies and compares this
   type directly: `scv->cs_id = cs->id` in `kernel/time/timekeeping.c:1439-1441`,
   `data_race(tk->cs_id) != id` in `kernel/time/timekeeping.c:905-906`, and
   `base->id != base_id` in `kernel/time/timekeeping.c:1451-1452`.

   The candidate (`src/include/linux/clocksource_ids.rs:11-20`) derives neither
   `Copy`/`Clone` nor `PartialEq`/`Eq`. Its value is therefore move-only and does
   not support the direct equality operations required by the Linux callers;
   notably, a normal Rust translation cannot copy `cs->id` out of a borrowed
   structure. The replacement representation must provide trivial-copy and
   integer-value equality behavior at the same program points as C, without
   introducing ownership, drop, panic, or allocation behavior.

3. **High — ABI/layout remains unproved and is still `PENDING_REVIEW` in the
   frozen records.**

   `rewrite/ABI.tsv` and `rewrite/LIFETIMES.tsv` retain `PENDING_REVIEW` for
   `enum clocksource_ids` on both x86_64 and aarch64. This is material: the C
   enum is embedded in the layout-sensitive structures listed above and is used
   by pointer in the external declaration
   `kvm_arch_ptp_get_crosststamp(..., enum clocksource_ids *cs_id)`
   (`include/linux/ptp_kvm.h:18-20`). The candidate's `#[repr(C)]` alone does not
   document or bind the exact signedness, width, alignment, field offsets, or
   allowed raw-value behavior required by the frozen C compilation ABI. Resolve
   those records from the pinned configuration/toolchain evidence before closing
   the task, then encode and document the resulting representation for both
   architectures.

## Review result

Reject pending correction and ABI-record resolution. No compiler, formatter,
rust-analyzer diagnostic, build, test, or runtime command was used.
