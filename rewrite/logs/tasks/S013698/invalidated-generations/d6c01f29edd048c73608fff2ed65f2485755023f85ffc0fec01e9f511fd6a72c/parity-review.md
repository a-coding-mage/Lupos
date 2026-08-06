# Parity review — S013698

Reviewed only the pinned source `vendor/linux/include/linux/device-id/auxiliary.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the fresh candidate
`src/include/linux/device-id/auxiliary.rs`, frozen task/manifests, and the
local upstream consumers needed to establish macro and byte semantics.  The
task is `common`, selected for both frozen targets; its recorded commands set
`__KERNEL__`, target `aarch64-linux-gnu`/`x86_64-linux-gnu`, and
`-funsigned-char`.

## Findings

1. **P1 — `auxiliary_device_id::name` loses the frozen unsigned-byte
   semantics.**  Upstream line 13 declares `char name[AUXILIARY_NAME_SIZE]`.
   Both frozen command records in `rewrite/FILE_MAP.tsv` for this header carry
   `-funsigned-char`, so this is an array of unsigned character values for
   both selected configurations.  Candidate line 16 instead uses
   `[core::ffi::c_char; AUXILIARY_NAME_SIZE]`, which neither encodes nor binds
   the required compiler-flag-selected unsignedness.  This is semantic, not
   merely layout: upstream `drivers/base/auxiliary.c:180` evaluates
   `id->name[0]`, and the identifier contents are character data.  Use an
   explicitly unsigned byte representation (while retaining the C layout) or
   an equally explicit frozen-target alias.

2. **P1 — `AUXILIARY_MODULE_PREFIX` is not a C string literal equivalent.**
   Upstream line 10 defines the preprocessor replacement token
   `"auxiliary:"`; as a C string literal it has static character-array
   storage and an implicit trailing `\\0`, and supports adjacent-literal
   concatenation.  Candidate line 11 changes it to a Rust `&str`, a fat
   reference whose string slice has no terminating NUL and cannot be used as
   the upstream macro token in concatenation.  The distinction is operative:
   `drivers/base/auxiliary.c:206` passes the macro to `%s`, and
   `scripts/mod/file2alias.c:1349` expands it as
   `AUXILIARY_MODULE_PREFIX "%s"`.  Represent the macro so callers retain
   the required NUL-terminated bytes and account for its literal-token use;
   a bare `&str` is not source parity.

3. **P2 — `AUXILIARY_NAME_SIZE` changes the operative macro’s integer
   type.**  Upstream line 9 is the unsuffixed C integer constant `40` (type
   `int` in this configuration), while candidate line 10 exports a `usize`
   constant.  The current direct upstream use at line 13 is an array bound,
   where the magnitude is the same, but `SYMBOLS.tsv` marks this an operative
   macro for both architectures.  The Rust representation must not silently
   change its width, signedness, and integer-conversion behavior for consumers
   that use the exported definition outside that one array-bound context.

## Checked parity items

- The candidate has the required immutable provenance for S013698, pinned SHA,
  and `common` architecture membership.
- The `kernel_ulong_t` alias corresponds to upstream’s `unsigned long` under
  the selected `__KERNEL__` condition; both frozen task command records define
  that condition.
- `#[repr(C)]` preserves the declared field order of
  `struct auxiliary_device_id`; no upstream packing/alignment attribute is
  present in this header.  This does not resolve finding 1’s element-value
  semantics.
- The include guard is preprocessing-only and has no separate Rust runtime or
  ABI analogue.

No compiler, formatter, rust-analyzer diagnostic, build, test, or runtime tool
was invoked.  No source file or queue/event file was edited by this review.
