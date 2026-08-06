# Resolution: S018288

Applied by the independent applier against pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

## Review dispositions

### Parity review — no findings

Confirmed. `security/selinux/include/policycap.h:7-24` contains exactly the
fifteen sequential anonymous-enum enumerators represented by the public `i32`
constants, ending in `__POLICYDB_CAP_MAX = 15`. The derived source macro at
line 25 remains `__POLICYDB_CAP_MAX - 1`, therefore the Rust
`POLICYDB_CAP_MAX` remains 14. The source declaration at line 27 has the same
symbol spelling and fixed extent as the Rust immutable foreign static.

### R1 — `char` signedness in the foreign array declaration

**Disposition: fixed.** The prior `*const c_char` declaration depended on
Rust's target-default C-char signedness. The frozen selected consumer command
in `rewrite/kbuild/x86_64/security/selinux/.avc.o.cmd` uses
`--target=x86_64-linux-gnu` and `-funsigned-char`; `FILE_MAP.tsv` binds this
header to that `security/selinux/avc.o` consumer. Thus the `char` objects
named by the pointers in the pinned declaration are unsigned for this frozen
configuration. The import now uses `*const core::ffi::c_uchar`, while
preserving the immutable `extern "C"` static, exact
`selinux_policycap_names` linkage name, and the `__POLICYDB_CAP_MAX` extent.

The matching pinned definition in
`security/selinux/include/policycap_names.h:10-26` provides fifteen
NUL-terminated policy-capability names. Pinned consumers use them only as
read-only character strings and retain the fixed-array bound: see
`security/selinux/ss/services.c:2183-2189`,
`security/selinux/selinuxfs.c:1745-1749`, and
`security/selinux/ima.c:30-31,56`.

## Closed task semantic facts

- The header guard at `policycap.h:3-29` is a preprocessing inclusion guard,
  with no Rust runtime or exported ABI counterpart.
- The anonymous enum at lines 7-24 has no tag or object. Its selected
  enumerators are C `int` constant expressions in the frozen x86_64 target;
  they map to the explicit `i32` constants in declaration order. There is no
  ownership, lifetime, locking, allocation, or cleanup behavior.
- `POLICYDB_CAP_MAX` at line 25 is an operative derived macro, mapped without
  pre-evaluation to `__POLICYDB_CAP_MAX - 1`.
- The only declaration with external linkage at line 27 is
  `selinux_policycap_names`, an immutable array of fifteen immutable pointers
  to immutable unsigned-character data in this frozen Kbuild context. It has
  static program lifetime and is imported as an immutable foreign static;
  neither the array nor its pointees acquire Rust ownership.

The Phase 0 identity and the checked-out `vendor/linux` HEAD both name
`425f94c2954b1fe80ebdbf9b29854e89750355df`; the frozen identity records queue
fingerprint `af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`.
No compiler, formatter, linker, test, or runtime command was run.
