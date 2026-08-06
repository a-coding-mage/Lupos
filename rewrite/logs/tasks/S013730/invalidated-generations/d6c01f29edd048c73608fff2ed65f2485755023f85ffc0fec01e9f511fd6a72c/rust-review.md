# Rust review — S013730, attempt 3

Reviewed only `vendor/linux/include/linux/device-id/rpmsg.h`,
`src/include/linux/device-id/rpmsg.rs`, and the direct S013730 queue/scope,
symbol, ABI, lifetime, file-map, and frozen-configuration records. No compiler,
formatter, linker, test, or analyzer was invoked.

## Findings

### R1 — `RPMSG_DEVICE_MODALIAS_FMT` is not represented as the C macro expression (must fix)

Linux line 12 defines `RPMSG_DEVICE_MODALIAS_FMT` as the string literal
`"rpmsg:%s"`. In C this macro expands to a `char[9]` string-literal expression;
when used in an ordinary expression it decays to a single `char *`, while it
also retains array/literal semantics in contexts such as `sizeof` and indexing.
The Rust candidate instead exports `&[u8; 9]`. A Rust reference is a fat,
non-C-ABI value and is neither the literal array nor its thin C pointer decay.
It can therefore change both ABI-facing uses and expression semantics. Replace
it with a representation that preserves the intended literal/`char` data and
provide an explicit thin-pointer view at FFI use sites, rather than publishing a
fat slice reference as the macro value.

### R2 — `rpmsg_device_id.name` substitutes `u8` for C `char` without resolving signedness (must fix or source-evidence disposition)

Linux line 15 declares `char name[RPMSG_NAME_SIZE]`; the candidate declares
`[u8; RPMSG_NAME_SIZE as usize]`. The array size and byte layout are compatible,
and `#[repr(C)]` gives the expected field ordering/alignment with the following
64-bit `kernel_ulong_t` field. However, `u8` changes C `char` value semantics
for bytes with the high bit set. The direct ABI records remain `PENDING_REVIEW`
for this struct and provide no frozen signedness conclusion. The final mapping
must either use the frozen-target C `char` representation or explicitly resolve
and record the relevant signedness evidence; layout equivalence alone does not
justify the substitution.

## Confirmed items

- Both frozen configurations select 64-bit targets; mapping kernel-only
  `unsigned long kernel_ulong_t` to `u64` is structurally appropriate for this
  task's two targets.
- `RPMSG_NAME_SIZE` is the C unsuffixed decimal `int` literal `32`; the Rust
  `i32` value preserves its signed 32-bit value for this header's array-bound
  use. Its explicit cast to `usize` is local to the Rust array length and does
  not itself introduce a runtime conversion.
- `#[repr(C)]` is present. `Clone, Copy` add no fields, layout, drop behavior,
  or ABI behavior; this all-scalar C struct is copied by value in C as well.
- No unnecessary `unsafe`, panic path, allocation, or ownership/drop mechanism
  was introduced.

## Verdict

Reject pending resolution of R1 and R2. The source-only review is complete.
