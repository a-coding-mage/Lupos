# Rust review — S013727, attempt 2

- Reviewer: `rust_reviewer`
- Model: `gpt-5.6-terra`
- Reasoning effort: `high`
- Pipeline: `P01`
- Scope: source-only review of `src/include/linux/device-id/platform.rs` against
  `vendor/linux/include/linux/device-id/platform.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- No compiler, formatter, rust-analyzer, build, test, debugger, or runtime tool
  was used.

## Findings

1. **Medium — `PLATFORM_MODULE_PREFIX` no longer has the C macro's expression
   semantics.** Upstream defines it as the string-literal macro
   `"platform:"` (`vendor/linux/include/linux/device-id/platform.h:10`). That
   produces a NUL-terminated character array, decays to a C string pointer in
   expressions, and participates in adjacent-literal concatenation; the
   latter is used by `MODULE_ALIAS(PLATFORM_MODULE_PREFIX "dw-hdmi-cec")`
   (`vendor/linux/drivers/gpu/drm/bridge/synopsys/dw-hdmi-cec.c:360`) and by
   `module_alias_printf(..., PLATFORM_MODULE_PREFIX "%s", ...)`
   (`vendor/linux/scripts/mod/file2alias.c:962`). The candidate instead
   exposes `pub const PLATFORM_MODULE_PREFIX: &[u8; 10]` at
   `src/include/linux/device-id/platform.rs:15`: a Rust reference value which
   cannot be used as a literal fragment or as the C `%s` argument without a
   caller-specific conversion. Preserve the macro's string-literal/FFI and
   compile-time-concatenation contract rather than substituting a slice
   reference.

2. **Medium — `platform_device_id` has lost C's ordinary by-value copy
   semantics.** The upstream aggregate has only scalar/array fields
   (`vendor/linux/include/linux/device-id/platform.h:12-15`), so C permits
   assignment and independent by-value copies. It is used as entries of
   platform ID tables, while the platform core retains pointers to those
   entries (`vendor/linux/include/linux/platform_device.h:33,277`; table walk
   at `vendor/linux/drivers/base/platform.c:1145-1151`). The `#[repr(C)]`
   candidate declaration at `src/include/linux/device-id/platform.rs:21-25`
   supplies neither `Copy` nor `Clone`; assigning it in Rust moves it and
   prevents the continued use of the source value. Derive the trivial copy
   traits (without changing its representation) so value behavior remains
   available to Rust translations wherever the C aggregate is copied.

## Checked without finding a defect

- `kernel_ulong_t = u64` matches `unsigned long` for both frozen 64-bit
  targets (`vendor/linux/include/linux/device-id/platform.h:6`).
- `#[repr(C)]`, `[u8; 24]`, and the following `u64` preserve the source field
  order and the expected 24-byte offset / 32-byte aggregate layout on the
  approved x86_64 and AArch64 targets (`vendor/linux/include/linux/device-id/platform.h:12-15`).
- The candidate introduces no `unsafe`, fallible operation, allocation, or
  panic path in this file.

## Verdict

Changes required before acceptance. Both findings are source-level; this
review makes no build or test claim.
