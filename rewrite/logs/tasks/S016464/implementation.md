# Implementation: S016464

Translated `include/uapi/linux/virtio_ids.h` to
`src/include/uapi/linux/virtio_ids.rs` from pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The header has no conditional selected content beyond its C include guard. All
47 selected object-like macros are represented as public `core::ffi::c_int`
constants, preserving their C integer-literal category and exact values. The
two identifier families remain distinct: normal `VIRTIO_ID_*` device IDs and
`VIRTIO_TRANS_ID_*` transitional IDs. There are no data layouts, functions,
allocation, ownership, locking, or ABI records in this macro-only header.

No build, formatter, test, or runtime command was run.
