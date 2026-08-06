# Resolution: S018289

Applied by the independent applier against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Review dispositions

### Parity review — no findings

**Disposition: confirmed.** The complete definition at
`security/selinux/include/policycap_names.h:10-26` has one externally linked
immutable `const char *const` array, with extent `__POLICYDB_CAP_MAX` and
fifteen initializers. The candidate keeps its exact
`selinux_policycap_names` data-symbol spelling through `#[unsafe(no_mangle)]`,
uses the dependent S018288 bound of 15, and contains the same fifteen ASCII
names in source order. Each Rust byte string contains exactly one trailing NUL,
which is required by the selected consumers that use the pointers as C strings.

The selected direct consumer
`security/selinux/ss/services.c:2183-2189` uses the array's extent for its
logging loop and separately reports only out-of-range policy capabilities.
The matching read-only uses in `security/selinux/ima.c:30-31,55-61` and
`security/selinux/selinuxfs.c:1745-1751` confirm that element order, fixed
extent, and NUL-terminated pointer targets are operative. No configuration
conditional, executable control flow, allocation, locking, cleanup, branding,
or additional source definition occurs in the pinned header.

### Rust semantics review — no findings

**Disposition: confirmed.** `selinux_policycap_name` is a one-field
`#[repr(transparent)]` wrapper around `*const c_uchar`; therefore every array
slot has the size, alignment, and pointer representation of the C `const char
*` slot. The frozen x86_64 commands use `-funsigned-char`, so the candidate's
`c_uchar` pointee representation agrees with this selected C context rather
than relying on target-default `c_char` signedness. The immutable Rust static
models C's outer `const` pointer slots. Its `Sync` implementation is limited
to global immutable storage whose values are pointers to immutable static byte
strings; it neither creates references nor grants a mutation path.

The S018288 declaration imports the same unmangled symbol as an immutable
`[*const c_uchar; 15]` foreign static. That raw-pointer view and this
transparent wrapper view have the same array storage representation, while the
wrapper is required for this defined Rust static's `Sync` requirement. Neither
view transfers ownership of the array or its pointed-to strings.

## Closed task semantic facts

- The inclusion guard at `policycap_names.h:3-4,29` and the clang-format
  directives have only preprocessing/formatting effects, so they have no Rust
  runtime, linkage, layout, or ownership counterpart.
- `selinux_policycap_names` is the sole selected object definition. It has
  external data-symbol linkage, fixed extent 15, immutable pointer slots, and
  static-program-lifetime pointee data. Its candidate mapping preserves those
  ABI and lifetime facts without creating a Rust-owned allocation.
- The bound comes from S018288's complete anonymous-enum mapping:
  `__POLICYDB_CAP_MAX` is the C `int` value 15 and is used as an array extent,
  while `POLICYDB_CAP_MAX` remains the distinct final valid index 14. There is
  no additional local type, macro, locking, RCU, refcount, or cleanup semantic
  to carry over from this header.
- All fifteen literals are byte-for-byte ASCII spellings of the pinned source,
  in declaration order: `network_peer_controls` through `bpf_token_perms`.
  Their backing byte-string storage is immutable and valid for the exported
  global's complete program lifetime.

The Phase 0 identity and `vendor/linux.SHA` bind this task to
`425f94c2954b1fe80ebdbf9b29854e89750355df`; the frozen queue fingerprint is
`af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.
No compiler, formatter, linker, test, runtime command, or diagnostic tool was
used. The candidate source required no change during application.
