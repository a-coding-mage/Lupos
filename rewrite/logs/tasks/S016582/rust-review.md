# Rust source review — S016582 / attempt 1 / P02

Reviewed only the pinned `vendor/linux/include/xen/interface/io/xenbus.h`,
the candidate snapshot, frozen task records, and direct pinned Xenbus users.
No compiler, formatter, test, analyzer, historical source, implementation
rationale, or parity report was used.

## Finding RUST-ENUM-FFI-001 — blocking: the public enum representation and value domain are not established

`enum xenbus_state` is not a Rust-local closed-state abstraction.  Pinned
`include/xen/xenbus.h:87,117,219,232` stores it in `struct xenbus_device`,
passes it through the `otherend_changed` callback, and exposes it through
function parameter/return interfaces.  More importantly,
`drivers/xen/xenbus/xenbus_client.c:945-958` declares an object of this type
and passes `&result` to `xenbus_gather(..., "%d", ...)`; the result is then
returned to callers, including the callback path in
`drivers/xen/xenbus/xenbus_probe.c:177-210`.  Thus the C code’s observable
contract includes an integer-form Xenstore input at this type boundary.

The candidate substitutes a fieldless Rust `#[repr(C)] enum`.  It records the
nine named discriminants, but it supplies neither source-proven frozen-target
size/alignment/signedness for the C named enum nor a representation that can
hold the complete integer value domain written/read through the C `%d` path.
Rust enum values have a restricted valid-discriminant domain; treating an
arbitrary integer supplied through this interface as this enum would create an
invalid Rust value rather than preserve the C integer behavior.  No validated
conversion, raw integer wrapper, or exact ABI record is present.  The frozen
ABI record remains `PENDING_REVIEW` for declaration, layout, alignment, and
export context, so source review cannot infer a safe replacement.

This blocks acceptance.  The applier must either establish the exact frozen
AArch64 C enum ABI and all inbound-value rules from pinned source/metadata and
then implement a representation preserving them, or leave the task blocked;
it must not assume that `#[repr(C)]` makes this Rust enum interchangeable with
the public C enum.

Affected semantic records:

- `SC1-41f521fad89bcad4608ec3825ad527f03cae4af6b8042ac3e77982ef9a2c7357`
  (`SYMBOLS.tsv`, `enum xenbus_state`, status)
- `SC1-88237288fb7ca23926550eb5273493d61ed4612856515bf84399482c6553ebfd`
  (`ABI.tsv`, export_kind)
- `SC1-fa18239ec80fbba005ca98b8f3686c647fceb1f8f075e288cbf4aceac2151f75`
  (`ABI.tsv`, layout)
- `SC1-d3e537526d4d0b839bce72e7577f7fe6414501ecd2502f4f64572f8ce17956b3`
  (`ABI.tsv`, alignment)
- `SC1-181dd64c94fa59486067bdb307727e109e543b7f3f7df861be7d1cec7538c84b`
  (`ABI.tsv`, status)

## Finding RUST-GUARD-002 — blocking: selected C header gate has no reviewed mapping

The selected `#ifndef/#define _XEN_PUBLIC_IO_XENBUS_H` at pinned source lines
10-11 is an operative macro/conditional in the frozen symbol inventory.  The
candidate omits it without a source-backed explanation of how its inclusion
and namespace behavior is preserved in the Rust module graph.  With the
macro’s selection expression and status still `PENDING_REVIEW`, that omission
cannot be closed as a behavior-neutral mechanical translation.

Affected semantic records:

- `SC1-c94e715723286cabcc6d3d1b5be1a0d4f00b8e82808242be4d8c1920637fdfed`
  (`SYMBOLS.tsv`, `_XEN_PUBLIC_IO_XENBUS_H`, selection_expression)
- `SC1-a6591d48f4161aef254a51807b4b4b8dbcfee4d2ea4c7c1e3579050947e99a99`
  (`SYMBOLS.tsv`, `_XEN_PUBLIC_IO_XENBUS_H`, status)

## Additional Rust-safety assessment

The candidate contains no pointer operations, `unsafe`, ownership-bearing
fields, allocation, callbacks, `Drop`, atomics, or synchronization to approve.
The enum/FFI defect is nevertheless sufficient to reject it because the named
type participates in stored state and callback/return ABI boundaries in pinned
Xenbus consumers.

Review result: **FINDINGS**.  No source mutation is approved.
