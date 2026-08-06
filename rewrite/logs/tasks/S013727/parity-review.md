# Parity review — S013727 / P01 / attempt 2

Role: parity reviewer  
Model: gpt-5.6-terra  
Reasoning effort: high  
Review basis: pinned `vendor/linux` revision `425f94c2954b1fe80ebdbf9b29854e89750355df`; `include/linux/device-id/platform.h`; frozen common x86_64/aarch64 task facts.  
Scope: source-only comparison of `src/include/linux/device-id/platform.rs`. No compiler, formatter, linker, test, debugger, or rust-analyzer diagnostics were used.

## Finding P1 — `PLATFORM_MODULE_PREFIX` loses C literal-composition semantics

**Severity: major.**

Pinned Linux defines the operative macro at `include/linux/device-id/platform.h:10` exactly as:

```c
#define PLATFORM_MODULE_PREFIX "platform:"
```

This is a string-literal token, not a standalone pre-terminated byte-array value. It is intentionally composable by adjacent C string-literal concatenation. The pinned source demonstrates that required behavior in `scripts/mod/file2alias.c:962`:

```c
module_alias_printf(mod, false, PLATFORM_MODULE_PREFIX "%s", *name);
```

and in `drivers/gpu/drm/bridge/synopsys/dw-hdmi-cec.c:360`:

```c
MODULE_ALIAS(PLATFORM_MODULE_PREFIX "dw-hdmi-cec");
```

After preprocessing, those form one C string literal; the only terminating NUL is the one supplied for the resulting complete literal. The runtime consumer in `drivers/base/platform.c:1409` likewise produces `MODALIAS=platform:<device-name>` by formatting `PLATFORM_MODULE_PREFIX` together with `pdev->name`.

The candidate instead defines at `src/include/linux/device-id/platform.rs:15`:

```rust
pub const PLATFORM_MODULE_PREFIX: &[u8; 10] = b"platform:\0";
```

That value already contains an interior/terminal NUL and is not composable as the C macro is. Concatenating it with a suffix yields `platform:\0suffix`, whereas the upstream literal composition yields `platform:suffix\0`. It therefore cannot preserve generated module aliases or modalias construction when the macro is used with a following literal/name. Replace the representation with one that preserves the prefix bytes and explicitly models the C call-site composition/termination rule; do not expose a pre-NUL-terminated slice as the macro's entire semantics.

## Compared items with no additional parity finding

- `PLATFORM_NAME_SIZE` retains the value 24, and `name` remains a 24-byte field at the first position of `platform_device_id`.
- Under both frozen 64-bit kernel configurations, Linux `unsigned long` for `kernel_ulong_t` is 64 bits; the candidate's `u64` `driver_data` retains the two-field order and 8-byte native alignment when paired with the 24-byte name array under `#[repr(C)]`.
- The `__KERNEL__` conditional is active in the frozen kernel compilation contexts; no additional configuration branch is omitted for this task's approved union.

## Disposition

One major parity finding remains. The candidate must be corrected and rechecked by the applier before this task can be accepted.
