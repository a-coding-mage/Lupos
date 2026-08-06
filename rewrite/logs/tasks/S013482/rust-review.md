# Rust semantics review — S013482

Reviewed `vendor/linux/include/linux/audit_arch.h` against
`src/include/linux/audit_arch.rs`, with source-level definition and caller
context from `vendor/linux/lib/compat_audit.c` and `vendor/linux/lib/audit.c`.
No compiler, formatter, linker, test, or diagnostics were used.

## Findings

### R1 — incomplete C arrays are exposed as scalar Rust statics (must fix)

The five declarations at `include/linux/audit_arch.h:27-31` are incomplete
array declarations (`extern unsigned int name[]`), not scalar declarations.
Their selected definitions in `lib/compat_audit.c:7-30` are complete,
variable-length `unsigned int` arrays terminated by `~0U`.  The selected
aarch64 caller in `lib/audit.c:75-79` passes each array expression to
`audit_register_class`, so C supplies the address of element zero; it does not
read or write a scalar object.

The candidate declares every symbol as `pub static mut name: u32`.  Although
this happens to request the same linker symbol address, it changes the Rust
surface into a one-element scalar: it permits scalar reads/writes and fails to
express that the symbol is address-only array storage of externally determined
extent.  It also invites a Rust caller to form a reference to a scalar object,
which is not the C declaration's contract and is too strong for globally
shared mutable storage.

Bind each symbol as an external zero-length array (for example,
`pub static mut compat_write_class: [u32; 0];`) and require consumers to form
a raw element pointer from `core::ptr::addr_of_mut!(compat_write_class).cast()`
when preserving C array-to-pointer decay.  Do not provide scalar static access
or a shared/mutable Rust reference as a replacement.  The existing `static
mut` is appropriate because the C definitions are non-const, but every actual
access must remain explicitly unsafe and raw-pointer based.

### R2 — upstream SPDX/copyright provenance is not retained (must fix)

The pinned header begins with `SPDX-License-Identifier: GPL-2.0-or-later` and
its Red Hat 2021 copyright/author notice.  The candidate instead says
`GPL-2.0-only` and drops the relevant notice.  This changes the declared
license and violates the required retention of upstream SPDX and relevant
copyright notices.  Retain the source's exact SPDX identifier and the
copyright/author notice after the immutable provenance header.

## Checked items

- `#[repr(C)]` requests the target C enum representation, and the seven named
  enumerators plus `AUDITSC_NVALS` retain their source order and values 0..7.
  The type is not used as an FFI parameter or result here; the C function
  declaration correctly uses `i32` for `int`, `u32` for `unsigned int`, and
  `extern "C"`.
- The enum lacks `Copy`/`Clone`, unlike a C enum's freely copyable scalar
  behavior.  Add these derives if this enum remains the selected Rust
  representation; alternatively, any integer-wrapper representation must
  preserve every C `int` value rather than creating invalid Rust enum values.
- Symbol spellings, public visibility, function unsafety, and the immutable
  task/source/revision/architecture provenance are otherwise present.  No
  `todo!`, `unimplemented!`, test configuration, or placeholder was found.

Result: changes required; do not accept the candidate until R1 and R2 are
resolved and the enum copy/validity decision is documented from the pinned
source/ABI record.
