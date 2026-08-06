# Resolution — S013711

Applier: P02, high-effort source adjudication only. I reopened the complete
pinned oracle, `vendor/linux/include/linux/device-id/i2c.h:1-19`, at Linux
revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, the leased candidate,
both independent review reports, and the task-local Phase 0 records. No
compiler, formatter, linker, test, emulator, debugger, rust-analyzer
diagnostic, historical Lupos source, or non-leased source edit was used.

## Review findings

| Finding | Disposition | Pinned evidence and resolution |
| --- | --- | --- |
| Parity P1 / Rust R4: required provenance SPDX value | Fixed | The immutable fresh-source provenance now begins `// SPDX-License-Identifier: GPL-2.0-only`, while the source/revision/architecture/task provenance lines are unchanged. |
| Parity P2 / Rust R3: `I2C_MODULE_PREFIX` was a named Rust reference | Fixed | Oracle line 12 is an object-like macro replacement for the literal token `"i2c:"`, not a declaration. `I2C_MODULE_PREFIX!()` now expands at each use to `b"i2c:\\0"`, exactly five C-string bytes, without an addressable header object, pointer/reference alias, linkage, or added data symbol. This preserves the literal-expression lowering required by source consumers such as `drivers/i2c/i2c-core-base.c:176,687` and the adjacent-literal form in `scripts/mod/file2alias.c:856-861`; translated format consumers compose the full `i2c:%s\\0` literal at their use site. |
| Parity P3 / Rust R2: `I2C_NAME_SIZE` widened C `int` to `usize` | Fixed | Oracle line 11 is unsuffixed `20`, hence a signed C `int` expression on both frozen targets. `I2C_NAME_SIZE!()` expands to `20i32`; the only header array bound explicitly converts that expression to `usize`. No `usize` constant is substituted for the macro. |
| Parity P4 / Rust R1: ordinary C aggregate copies became Rust moves | Fixed | Oracle lines 14-17 declare a resource-free C aggregate. `#[derive(Copy, Clone)]` restores ordinary by-value copy behavior without allocation, `Drop`, ownership, or a layout change. The `#[repr(C)]` field order remains `name` then `driver_data`. |

## Final semantic-record closure

The Phase 0 TSVs are frozen evidence and this applier's authorized edits are
limited to the leased source and this resolution. The following source-backed
dispositions close every S013711 `PENDING_REVIEW` item for both `x86_64` and
`aarch64` in the task evidence:

| Record(s) | Final disposition |
| --- | --- |
| `LINUX_DEVICE_ID_I2C_H`, `#ifndef`, and matching `#endif` | C preprocessing-only include-once control. It has no Rust runtime item, storage, linkage, ABI, ownership, or lifetime effect. |
| `__KERNEL__` conditional and `kernel_ulong_t` | Both frozen in-kernel command families select the `__KERNEL__` branch at oracle lines 5-7. The selected `unsigned long` typedef is LP64 `u64`, size/alignment 8, with no storage, linkage, ownership, lifetime, locking, RCU, refcount, or drop behavior of its own. |
| `I2C_NAME_SIZE` | Compile-time signed-C-`int` expression value 20. It has no storage, linkage, or address identity; only the Rust array-bound context explicitly converts it to `usize`. |
| `I2C_MODULE_PREFIX` | Compile-time C string-literal expression with standalone bytes `i2c:\\0`. It has no declared header object, linkage, allocation, ownership, lifetime, synchronization, or pointer identity. Format-literal composition remains the responsibility of each source consumer. |
| `struct i2c_device_id` | `#[repr(C)]`, unsigned-byte `name[20]` at offset 0, four ABI padding bytes, `kernel_ulong_t` `driver_data` at offset 24, alignment 8, total size 32 on both frozen 64-bit targets. Frozen command metadata records `-funsigned-char`, so `name` is `[u8; 20]`, not a Rust string. The plain record is `Copy, Clone`; it has no pointer, callback, flexible/packed/union member, allocation, ownership transfer, lock, RCU, refcount, or independent drop contract. Storage duration and lifetime belong to each enclosing ID-table or local-object owner; `driver_data` remains opaque driver-private unsigned-word data. |

`DRIVER_ABI.tsv` and `BLOCKERS.tsv` have no S013711 row. No task-local
source, ABI, ownership, lifetime, locking, RCU, refcount, configuration, or
macro-mapping question remains unresolved.

## Final adjudication

The candidate now maps every operative declaration in the pinned 19-line
header: the selected kernel typedef, both macro replacement expressions, and
the C-layout, freely-copyable record. Both reviews' P1-P4/R1-R4 findings are
resolved. This task is ready for the atomic `DONE` transition only; that is a
source-pipeline conclusion, not a compile, link, test, boot, or runtime claim.
