# S000200 applier resolution

Pinned source reopened: `vendor/linux/arch/arm64/include/asm/vncr_mapping.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df` (SHA-256
`87882faa68e0cea46ad6a2e1cc1fa2d03a470b52a2f96e8cf864cab0c48ce3fd`).

## Review dispositions

| Finding | Disposition | Upstream evidence |
| --- | --- | --- |
| P1: SPDX identifier changed to `GPL-2.0-only` | Resolved: the Rust header now retains the exact `GPL-2.0` identifier. | `vncr_mapping.h:1` |
| Rust review: no Rust-specific finding | Accepted after independent recheck. The constants have no storage, address, FFI, layout, ownership, synchronization, or unsafe contract. | `vncr_mapping.h:10-113` |

## Independent parity recheck

- The upstream header has 104 unconditional `VNCR_*` object-like macros and
  the destination has 104 same-named `pub const` items. A name/value comparison
  found no missing, extra, or unequal numeric entry; hexadecimal case-only
  spelling differences leave the integer constant value unchanged.
- Every value is a positive, 8-byte-aligned byte displacement and fits the
  frozen-target C `int` range. Retaining each as `i32` preserves its signed
  integer value and avoids introducing a symbol or storage object.
- `arch/arm64/include/asm/kvm_host.h:447-451` derives the VNCR system-register
  enum offsets with `(VNCR_ ## r) / 8`; the preserved byte values therefore
  retain the upstream offset semantics, including every non-contiguous region.
- The source contains no configuration conditional around these definitions;
  its include guard has no Rust module analogue. There are no ABI, lifetime,
  locking, RCU, refcount, allocation, cleanup, or branding records to resolve.

The two reviews are resolved. No compiler, formatter, linker, test, emulator,
debugger, benchmark, or runtime command was run.
