# Rust review — S016143 (slot 2)

Result: **REJECT — the public enum representation must be corrected and its
target ABI decision closed before this task can be accepted.**

Reviewed the complete pinned
`vendor/linux/include/uapi/linux/hash_info.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the fresh candidate
`src/include/uapi/linux/hash_info.rs`, the S016143 scope/symbol/ABI/lifetime
records for both frozen targets, and pinned immediate consumers including
`include/crypto/hash_info.h`, `lib/crypto/hash_info.c`, and
`fs/ubifs/{ubifs.h,auth.c}`. No source, manifest, or queue file was edited;
no build, formatter, test, or runtime command was run.

## Findings

1. **High — `#[repr(C)] pub enum` narrows the C `enum hash_algo` value domain
   and changes the public enumerator-expression surface.**

   The source at `include/uapi/linux/hash_info.h:17-42` declares an ordinary
   C enum. Its named enumerators are C integer constant expressions, and an
   object of the enum's compatible integer type can carry values beyond the
   24 named values. This is observable in selected pinned code: the
   `enum hash_algo auth_hash_algo` field at `fs/ubifs/ubifs.h:1493` receives
   the `int` result of `match_string()` at `fs/ubifs/auth.c:267-268`; only
   afterward does line 269 cast it to `int` and reject the negative failure
   result. The C object therefore deliberately represents a negative,
   non-enumerator value during that path.

   The candidate's `src/include/uapi/linux/hash_info.rs:13-40` is a Rust
   nominal enum with only discriminants `0..=23`. Rust enum validity excludes
   every other `c_int` bit-pattern, so translating the pinned assignment into
   such a field would require constructing/holding an invalid Rust value.
   Its variants also are nominal enum values rather than the source's flat C
   integer constant expressions, forcing changed indexing/conversion behavior
   at consumers such as the `hash_algo_name[HASH_ALGO__LAST]` and
   `hash_digest_size[HASH_ALGO__LAST]` declarations in
   `include/crypto/hash_info.h:38-39` and their designated-initializer tables
   in `lib/crypto/hash_info.c:11-63`.

   Represent the tag/value domain with an integer representation that accepts
   every compatible C integer value (for example, a `c_int` alias with flat
   `c_int` constants, once the ABI finding below is resolved), rather than a
   Rust data-carrying/nominal enum. Preserve all 24 names and values,
   including the count sentinel value 23.

2. **Medium — the required frozen-target enum ABI record remains unresolved.**

   The S016143 rows in `rewrite/ABI.tsv` for `enum hash_algo` on both
   `x86_64` and `aarch64` remain `PENDING_REVIEW`. The candidate selects
   `#[repr(C)]` without evidence establishing the selected compiler's exact
   compatible integer type, size, alignment, and calling/layout behavior for
   this unforced C enum on each frozen target. C leaves the compatible integer
   type implementation-defined; the candidate's appropriate replacement must
   be justified by the pinned LLVM/configuration evidence and the ABI records
   must be completed before `DONE`.

## Checks that passed

- The source has no configuration branch besides its include guard; the
  candidate introduces no `cfg` divergence and correctly carries `common`
  x86_64/aarch64 provenance.
- SPDX (`GPL-2.0+ WITH Linux-syscall-note`), source path, revision, task ID,
  copyright notice, all 24 names, explicit values `0..=23`, ordering, and the
  `HASH_ALGO__LAST` terminal-count value match the pinned header.
- No unsafe code, FFI declaration, pointer/ownership operation, allocation,
  synchronization primitive, panic/unwrap path, placeholder, or Rust test is
  introduced by this declarative candidate.

No source edits were made by this reviewer.
