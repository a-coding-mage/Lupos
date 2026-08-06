# Resolution — S013721

Applier: P01, high-effort source adjudication only.  The pinned oracle is
`vendor/linux/include/linux/device-id/mhi.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, lines 1--24.  The queue lease is
S013721/P01 attempt 1 on `feat/bun-like-rewrite-test`; the frozen queue
fingerprint verifies.  No compiler, formatter, linker, test, emulator,
debugger, rust-analyzer diagnostic, historical Lupos source, or non-leased
source edit was used.

## Review findings

| Finding | Disposition | Pinned evidence and resolution |
| --- | --- | --- |
| Parity P1 / Rust 1: modalias macros became public statics and pointer aliases | Fixed | Oracle lines 9 and 12 are object-like macro replacements for the respective string-literal tokens, not declarations.  `MHI_DEVICE_MODALIAS_FMT!()` and `MHI_EP_DEVICE_MODALIAS_FMT!()` now expand directly to `b"mhi:%s\\0"` and `b"mhi_ep:%s\\0"`.  The exact byte literals retain their NUL terminators (7 and 10 bytes), while the candidate declares no header static, raw-pointer alias, or added data symbol.  The resulting expression mapping is what translated consumers use locally for format/uevent bytes; no C-style named storage/address identity was invented.  Source consumers confirm both forms: host `init.c:1423`, endpoint `main.c:1677`, and `scripts/mod/file2alias.c:1323,1331`. |
| Parity P2 / Rust 2: `MHI_NAME_SIZE` widened from C `int` to `usize` | Fixed | Oracle line 10 is unsuffixed `32`, therefore C `int` for both frozen targets.  `MHI_NAME_SIZE!()` now expands to `32i32`; the sole Rust array-bound use explicitly converts it to `usize`, preserving the source macro's arithmetic type apart from that Rust-only bound conversion. |
| Parity P3: `const char chan[32]` was a writable public field | Fixed | Oracle line 20 makes each byte of the inline array const-qualified; matching tables are `static const` and use `id->chan[0]` as their all-zero terminator (`drivers/bus/mhi/host/init.c:1431+`, `drivers/bus/mhi/ep/main.c:1685+`; table initializers include `drivers/net/mhi_net.c:388+` and `net/qrtr/mhi.c:159+`).  The ABI field remains the first 32 `u8` bytes but is private, and `mhi_device_id::chan()` exposes only a shared read-only array reference.  `driver_data` remains the only mutable public member, matching its non-const declaration.  The accessor is a Rust visibility adaptation only: it has no C linkage, symbol, storage, or layout effect. |
| Rust 3: required provenance SPDX value differed | Fixed | The immutable provenance line now uses `// SPDX-License-Identifier: GPL-2.0-only` exactly as required by the rewrite protocol.  The pinned upstream source SPDX remains recorded by provenance rather than being represented as a mutable completion claim. |

## Final semantic-record closure

The Phase 0 TSVs are frozen evidence and this applier's authorized changes are
limited to the leased source and this resolution.  The following source-backed
dispositions close every S013721 `PENDING_REVIEW` item for both `x86_64` and
`aarch64` in the task evidence:

| Record(s) | Final disposition |
| --- | --- |
| include guard and its `#ifndef`/`#endif` records | C preprocessing-only include-once control.  It has no runtime or Rust ABI counterpart. |
| `__KERNEL__` conditional and `kernel_ulong_t` | Both frozen kernel command families select `__KERNEL__`; `kernel_ulong_t` is an unsigned 64-bit LP64 word (`u64`), size/alignment 8, with no storage, linkage, ownership, lifetime, or synchronization behavior of its own. |
| modalias macros | The two macros are compile-time literal expressions only.  Their characters and terminating NULs are `mhi:%s\\0` (7 bytes) and `mhi_ep:%s\\0` (10 bytes); they create no declared global object, linkage, or pointer alias. |
| `MHI_NAME_SIZE` | C signed `int` expression value 32; only the struct bound makes an explicit Rust `usize` conversion. |
| `struct mhi_device_id` | `#[repr(C)]`: unsigned-char `chan[32]` at offset 0, `kernel_ulong_t` at offset 32, alignment 8, total size 40 on both frozen 64-bit targets.  It has no packed/union/flexible member, function, section annotation, exported symbol, allocation, ownership transfer, lock, RCU, refcount, callback, or drop contract.  Storage duration and enclosing lifetime belong to each table/object owner.  `driver_data` is an opaque integer word; later pointer/integer conversions must establish their own provenance at the conversion site. |

`-funsigned-char` for the selected C command families is recorded in the
frozen mapping evidence cited by the reviews; consequently `chan` uses `u8`,
not a Rust string or signed character array.  The direct MHI headers retain
only pointers to `const struct mhi_device_id` (`include/linux/mhi.h:481--520`,
`include/linux/mhi_ep.h:195--215`).  No driver source was translated or
modified.

## Final adjudication

The candidate now translates every operative declaration in the 24-line
oracle without P1--P3 or the Rust-review provenance defect.  No unresolved
semantic, ABI, ownership, lifetime, lock, RCU, or configuration question
remains for S013721.  It is ready for the atomic `DONE` transition only; this
is a source-pipeline conclusion, not a compile, link, test, boot, or parity
execution claim.
