# Rust semantic review — S013482 / P02 / attempt 1

Reviewed only the pinned `include/linux/audit_arch.h` at Linux
`425f94c2954b1fe80ebdbf9b29854e89750355df`, its fresh candidate, frozen
Phase 0 records, and direct pinned consumers/definitions.  No compiler,
formatter, linker, test, rust-analyzer diagnostic, or historical Lupos source
was used.

Result: **FINDINGS — reject candidate as written.**

## RUST-ENUM-INT-DOMAIN — high

`src/include/linux/audit_arch.rs:7-29` translates the C enum and its
enumerators into a Rust `#[repr(C)]` enum and enum values.  That is not the
contract exposed by this header's users.  In C, each `AUDITSC_*` enumerator is
an integer constant expression; `audit_classify_compat_syscall()` is declared
to return `int` in the header at line 24, and the selected implementations
return these constants through that `int` result.  `kernel/auditsc.c:159-193`
then switches on that integer result, including a default case.

The candidate's re-exported `AUDITSC_*` values instead have Rust
`auditsc_class_t` type.  They cannot be used in the same integer expressions,
integer-return paths, or `i32` matches without new casts, and the Rust enum
also imposes a closed discriminant domain that the C integer interface does
not.  `AUDITSC_NVALS` must retain its integer-count role as well.  The
candidate therefore changes both the namespace/type contract and the
integer-promotion behavior.  Preserve these exported values as explicitly
sized C-compatible integer constants (and preserve any required tag ABI only
without narrowing the ordinary integer interface).

Affected closure evidence: enum type and enumerator selection/ABI contract on
both architectures, including SC1 keys
`SC1-7c435d078dec4a74d693693813b6e67815271aace4f2c31b4d37b51432def825`,
`SC1-c1c87e5b6750a203d91bd84a7ae6b1f9ef8b51eb67a32a8b6ebeb8e71ec5c18d`,
`SC1-8c27519e1b7f0e14d50261c715dd579193ed7ceb4b1e83bc74d2d90df647f347`,
and `SC1-b61ff480b2104634b017d2e86c30a3ca5e4e661676cee4815e78db3e7f0d4d10`.

## RUST-INCOMPLETE-ARRAY-ABI — high

`src/include/linux/audit_arch.rs:34-38` declares every C incomplete array as
`[u32; 0]`.  A zero-length Rust array is not C's `extern unsigned int name[]`:
it promises zero elements, makes normal indexing a guaranteed bounds panic,
and gives later Rust consumers no faithful way to perform C's pointer-style
indexing over the architecture-defined object.  The source intentionally
leaves the extent incomplete at the declaration site.

This is operative, not hypothetical.  For the frozen AArch64 configuration,
`CONFIG_AUDIT_COMPAT_GENERIC=y` selects `lib/compat_audit.c`; lines 7-30 define
all five arrays from generated syscall-class lists followed by `~0U`, and
`lib/audit.c:74-80` passes their base addresses to `audit_register_class`.
Their actual nonzero lengths are architecture/generated-header dependent.  On
x86_64 the selected IA32 path uses its own `ia32_*` arrays, which further shows
that the header declaration must not invent one fixed Rust extent.  Although
`u32` has the intended unsigned-32-bit element width for the approved ABIs,
the candidate's zero extent breaks object-size/access semantics; a raw
extern-object/pointer representation or an exact per-architecture extent is
required, with raw access kept at an explicit documented unsafe boundary.

Affected closure evidence: the five external-array declarations on both
architectures, including SC1 keys
`SC1-8196f5f1c271c95f907c374017fa261d0c36fb6bf871962d717d142536faa96a`,
`SC1-04985af46d755fac2df067901e81886b9a478d1944643e556eeb7f7fd7d8ad19`,
`SC1-a771f5443251952ca228e625eeacd9cfa1d6f05aa17dc0359fd825bbac98fa8f`,
`SC1-4a68306c50fc91220f2084f7f22bf4b7cf941847cd045dcd98254a7cba54de3c`,
`SC1-78b3b9214a208833311e2379a847624bd3ca1c018205e0d9f11a5be68a457f09`,
`SC1-469220a3e659dfddde94aa56247034545e98ba7e921c58f43e0e1c29a49f66a5`,
`SC1-3363b32ffa428430a8bb654d7fc244525c6851e6938b828945eeacaf7fc94985`,
`SC1-3a988f9e072036608ab851cc97cb5c52686639098d2c54b195213074eac3b44d`,
`SC1-10495e76417fac08393f413cc8431c4270005f7f45debaa805e86536a53345d5`,
and `SC1-c8cc8fb63c50f00a3004f91bb9fb7bb4162a3865be226e67e2f91fef33df3cee`.

## Other source-review observations

- The foreign function signature at line 32 maps C `int`/`unsigned int` to
  `c_int`/`u32`, and `extern "C"` is the correct calling convention for both
  approved targets.  This does not cure the enum-constant or array failures.
- There are no explicit `unsafe` blocks, allocations, callbacks, drops,
  pinning, atomics, or borrow-bearing references in the candidate.  Future
  access to a mutable external class table must nevertheless remain raw and
  synchronized according to the Linux initialization/registration protocol;
  the current zero-length declarations cannot express that access faithfully.
- The pinned header has only its include guard and no Kconfig conditional
  branches.  The direct Rust mapping is correspondingly unconditional.  The
  `AUDIT_ARCH*` bit-composition macros belong to `include/uapi/linux/audit.h`,
  not this leased header, and are not present in this candidate/source pair.
- No panic, bounds, or allocation behavior exists in the C declaration-only
  header.  The fabricated `[u32; 0]` bounds behavior is itself the relevant
  introduced panic/semantic divergence.
