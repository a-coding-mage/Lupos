# Parity review — S000803

Reviewed `src/arch/x86/include/uapi/asm/unistd.rs` against pinned
`vendor/linux/arch/x86/include/uapi/asm/unistd.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 scope,
symbol inventory, and header-closure records.  This was a manual source
review only; no compiler, formatter, linker, test, or runtime command ran.

## Finding P1 — upstream UAPI SPDX exception was changed

`arch/x86/include/uapi/asm/unistd.h:1` is
`GPL-2.0 WITH Linux-syscall-note`.  Candidate line 1 instead says
`GPL-2.0-only`.  `rewrite/BRANDING_ALLOWLIST.tsv` has no entry authorizing a
license-identifier change.  This loses the upstream syscall-note exception
for a UAPI header and violates the requirement to retain SPDX identifiers.

Disposition required: restore the upstream SPDX identifier in the translated
file while retaining the immutable source/revision/architecture/task
provenance fields.

## Finding P2 — bitwise-behavior claim is broader than the Rust declaration

The pinned macro is the integer literal `0x40000000`, whose C type is `int`
on the frozen x86 target.  The candidate's `i32` value and bit pattern are
correct.  However, Rust has no C usual-arithmetic conversions: an expression
analogous to the source comment's unsigned-long `nr & ~__X32_SYSCALL_BIT`
does not combine a `u64` operand with this `i32` constant without an explicit
conversion at the use site.  Candidate lines 9--10 therefore overstate what
the declaration alone preserves.

Disposition required: narrow/correct that documentation and ensure later
ported use sites explicitly reproduce C's conversion/sign-extension rule;
do not change this public constant away from `i32` merely to make mixed-width
Rust expressions convenient.

## Conditional and generated-header assessment

The frozen x86_64 header consumer command records `-D__KERNEL__` and
`--target=x86_64-linux-gnu`.  Consequently the source's `#ifndef __KERNEL__`
body, including the `__i386__`, `__ILP32__`, and fallback
`asm/unistd_{32,x32,64}.h` selections, is inactive in the selected kernel
translation.  The header-closure inventory classifies generated
`unistd_32.h` and `unistd_64.h` as `BUILD_METADATA`; it gives neither a Rust
translation task.  The candidate correctly introduces no Rust substitute
for those inactive user-header includes.  The C header guard likewise has no
additional run-time or ABI object beyond the Rust module's single definition.

No copyright notice appears in the pinned source beyond its SPDX line.  The
candidate provenance fields name the correct source path, pinned revision,
architecture, and task ID.
