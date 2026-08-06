# Parity review — S016002 (slot 1)

## Scope and evidence

- Pinned source: `vendor/linux/include/uapi/asm-generic/errno-base.h`
- Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Candidate: `src/include/uapi/asm-generic/errno-base.rs`
- Queue scope: common (`x86_64,aarch64`)

## Result

No parity findings.

The upstream header contains one include guard and exactly 34 unconditional
object-like errno macros.  The candidate preserves each public name in source
order and its exact decimal value: `EPERM` through `ERANGE` map consecutively
from 1 through 34.  Every upstream replacement token is an unsuffixed decimal
integer literal; representing the constants as `core::ffi::c_int` preserves
the frozen targets' C `int` value type.  The header's C preprocessor include
guard has no separate Rust declaration and is correctly not represented as a
runtime value.

The candidate retains the upstream SPDX expression and has the required
source, revision, architecture, and task provenance.  It introduces no
configuration branch, storage, linkage, driver code, branding delta, or test.
