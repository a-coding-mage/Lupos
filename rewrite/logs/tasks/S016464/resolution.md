# Resolution: S016464

Reopened pinned `vendor/linux/include/uapi/linux/virtio_ids.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` and the candidate, both review
reports, and task evidence.

## P1 — upstream BSD notice was not retained

Resolved.  Restored the complete upstream `Virtio IDs` BSD notice verbatim as
a Rust block comment immediately after the immutable provenance block.  This
retains the IBM attribution, source and binary redistribution conditions,
endorsement restriction, and complete warranty/liability disclaimer.  The
immutable GPL provenance line is unchanged.

## Rust review result

Accepted.  The header's 47 unsuffixed C integer literals remain one-for-one
public `core::ffi::c_int` constants.  `c_int` remains the correct signed C
`int` type for both approved targets; no identifier name, order, literal,
layout, ownership, synchronization, or unsafe behavior changed.

No build, formatter, test, or runtime command was run.
