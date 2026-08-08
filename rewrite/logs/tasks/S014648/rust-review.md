# Rust review — S014648 attempt 1 (P02)

Reviewer: `rust_reviewer` (`gpt-5.6-terra`, high)

Scope inspected independently: pinned `vendor/linux/include/linux/pinctrl/pinctrl-state.h`, the candidate snapshot, selected direct pinned consumers in `include/linux/pinctrl/consumer.h` and `drivers/base/pinctrl.c`, and the frozen scope/symbol/ABI/lifetime records.  I did not inspect implementation rationale, parity review, historical source, or compiler-derived diagnostics.  No compiler, formatter, test, analyzer, or runtime command was run.

## Finding P1-RUST-FFI-MACRO-LITERALS — reject

Linux defines each `PINCTRL_STATE_*` name as a C string-literal macro.  The literal includes its terminating NUL when it is used as an expression, then decays to `const char *`; direct consumers pass it to `pinctrl_lookup_state(..., const char *name)` (`include/linux/pinctrl/consumer.h:181-198`, `205`) and the device-core uses the same macros in that interface (`drivers/base/pinctrl.c:41-79`).

The candidate instead publishes `&'static str`.  That value is a Rust UTF-8 slice (data pointer plus length), does not contain a C NUL terminator, and cannot be passed through the Linux-facing `const char *` ABI without a separate conversion/allocation/provenance rule that this file does not provide.  Such a conversion would also alter the macro's static-literal storage and FFI contract.  There are no unsafe blocks to audit, but that does not make the missing raw-pointer/C-string representation sound.

Affected semantic records: `SC1-6dc4bbd4b4b271f9ee55663a43c22eaf704c94fcd140a3c7f229e3298b869a1b`, `SC1-8f2ecca135d49559fdcd6ebea00c0313621416726d1d7f14e1cdd85bfda904fe`, `SC1-1e08ac9572e377b3e0f089afa6a3fefa174313b3e35e937b3c103ad11cd8d3c7`, `SC1-57928ab03858b98e5bc7b413a2aa0686515a0760a777106b3ed39901fecfca5a`, `SC1-e6ebef10f017625da7b649b742f88149a99f08269eb7ad2e185d1e7dab4a3a5e`, `SC1-076d8e325b98a6c9805edac92b300c4a3151a48c4af7c77e09cc21ab4326a864`, `SC1-7b3f34382413f28447417ac564d23c0547dd2c70dd93884f6a46849a3a543038`, `SC1-2ea324141317df35e5a525938408400627bbdbe457e4b2af9eb63d47f1030d59`.

## Finding P1-RUST-MACRO-EXPANSION — reject

The original names are preprocessor substitutions, not C objects.  This is materially observable: `drivers/i2c/i2c-core-base.c:327` uses `PINCTRL_STATE_DEFAULT " state not found for GPIO recovery\\n"`, relying on adjacent C string-literal concatenation.  A Rust `pub const &str` cannot substitute into this expression form or preserve its compile-time token semantics.  The candidate contains no frozen, source-proven module/macro mechanism that preserves both that use and the header's selected include-guard macro.

The source and frozen records therefore do not establish an exact Rust representation of these operative macros.  The semantic proposal must not close these entries as `COMPLETE` on the basis of the candidate.

Affected semantic records: `SC1-ad64c68c04fb0967f9384be03e5d6b4ebf66bd7f54c83877bb2d291efcff8678`, `SC1-6d53d7dcd03746b8b3e34dc1d56cee398073b4e4f8ac966e4a6071278e04cc12`, plus the eight `PINCTRL_STATE_*` selection-expression records listed above.

## Audit notes

The header itself declares no callbacks, structs, ownership-bearing objects, atomics, pinning, `Send`/`Sync` boundary, drop path, or unsafe operations.  The rejection is specifically the candidate's changed ABI/value representation and lost macro-expansion behavior.  No source-only path visible in the pinned contexts supplies the missing cross-file C-string and macro-preservation contract, so this review cannot approve it as a Rust-only substitution.
