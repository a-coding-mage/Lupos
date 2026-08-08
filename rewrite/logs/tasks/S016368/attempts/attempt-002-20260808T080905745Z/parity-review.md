# Parity review — S016368 (attempt 2, slot 1)

Result: **FINDINGS**. This was a manual source-only review. No compiler,
formatter, test, linker, rust-analyzer diagnostic, or historical Lupos source
was used.

Reviewed inputs: pinned `vendor/linux/include/uapi/linux/securebits.h`, current
`src/include/uapi/linux/securebits.rs`, the current candidate diff, and the
direct frozen scope, file-map, symbol, ABI, lifetime, branding, and sealed
semantic-closure records for S016368.

## Findings

### F001 — `issecure_mask(X)` is not available to the header's downstream users

Linux symbol: `issecure_mask(X)`.

The pinned UAPI header defines this operative macro at
`include/uapi/linux/securebits.h:9`. It is available to every C translation
unit which includes the header. The immediate local wrapper
`include/linux/securebits.h:5-7` includes that UAPI header and defines
`issecure(X)` in terms of `issecure_mask(X)`. Pinned caller
`security/commoncap.c:994`, `:1394`, and `:1396` also use the macro to alter
`cred->securebits`.

The candidate's `macro_rules! issecure_mask` at
`src/include/uapi/linux/securebits.rs:13-17` is neither exported nor otherwise
made available outside that module; every candidate use is internal, in the
constant initializers at lines 24-68. A sibling or parent translation of the
local wrapper/callers has no source-visible `issecure_mask` mechanism to use.
Thus the candidate preserves the presently enumerated constant values but
omits the selected reusable macro/interface and cannot support the established
downstream behavior. Provide a source-visible exact equivalent with the
required single evaluation and signed 32-bit mask semantics, then re-review.

Affected sealed closure keys:
`SC1-92239f10b9eaa3d897af9aca79705572e5216edf5c36e59ddc9ffccf47954e8d`,
`SC1-717a73bb9bd61b7dc369dc90051c18e79670c774d777270be83d1e2f538c0088`,
`SC1-d5032262e96fcf2fe00cabca2035a1664cbee64759104c8136f699a769fb1a79`, and
`SC1-20a010570719de1a4ebfce6715bb0bbfdc89d3a28c5a6336dffe33070f9fb74c`.

### F002 — `_UAPI_LINUX_SECUREBITS_H` and its selected conditional closure lack a source-backed mapping

Linux symbol: `_UAPI_LINUX_SECUREBITS_H` (with the `ifndef@2` / `endif@83`
conditional pair).

The pinned header's lines 2-3 and 83 implement the UAPI include guard. The
frozen symbol inventory selects both conditionals and the guard macro for both
architectures. The candidate has no corresponding definition, guard, or
documented Rust-module equivalence. Rust module loading may make a literal
C-preprocessor guard unnecessary, but the current candidate and frozen direct
records contain no source evidence establishing that equivalence. It is
therefore not valid to mark these selected records `COMPLETE`; resolve and
record the mapping before final semantic closure.

Affected sealed closure keys:
`SC1-2ffe97a1514663eb3f4d5abd65b752e432fb609ba544c1cd9a8e592ab433eca6`,
`SC1-b581e6fcd5f97ed12badad46341dafe4b605d35dd2ce12f2f39959edac1f1611`,
`SC1-ea961094d435ab95095321ad01fe1f3ad36887bd229867776c8ac05ab51b34b7`,
`SC1-26ea710f0edadc4c84c782be179d553c62036b58c2dc9db9685136efc12d2d64`,
`SC1-6a36db6251490a1a9363f045c59d4ce5b9c71da71b5d7ea8bb7e50d2689743fd`,
`SC1-9e149fdc3cdcec105aa31fd5da6c9649eecc68eb6626aaf02b98eea39efd8891`,
`SC1-1f7f9381ffe9c363ca1aeade490a041b87bbe5456e11d2c803396c63c50191ee`, and
`SC1-595a04f8a1279f4d74221122652ca56e601869276815af65db00cb5b805e53b5`.

## Verified portions

For the defined securebit inputs 0 through 11, the candidate retains every
numeric setting selector, each `SECBIT_*` value, `SECUREBITS_DEFAULT`, and the
three aggregate expressions from pinned header lines 11 and 19-81. Their
declared `i32` type matches the C `int` result of the corresponding integer
constant/mask expressions for these selected values. No layout, linkage,
allocation, locking, refcount, RCU, error-path, or branding delta exists in
this macro-only header beyond the findings above.

The 125-row sealed proposal marks all rows `COMPLETE`; F001 and F002 mean that
the listed rows must not be accepted as closed by this review.
