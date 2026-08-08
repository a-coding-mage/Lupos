S000046 implementation attempt 1

Source: vendor/linux/arch/arm64/include/asm/compiler.h
Destination: src/arch/arm64/include/asm/compiler.rs
Linux revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
Architecture: aarch64
Pipeline: P01

The complete pinned source was inspected.  Its selected operative behavior is
not a data declaration: `xpaclri(ptr)` is a GNU C statement-expression that
binds an unsigned-long temporary to the fixed AArch64 register x30 and emits
inline assembly (`hint #7`) with a read-write `+r` constraint and an optional
`.arch` assembler preamble.  The two ptrauth stripping macros conditionally
expand to this statement-expression or to the original expression.  The file
also conditionally redefines the compiler builtin `__builtin_return_address`
to apply the kernel stripping operation.

The frozen AArch64 configuration enables CONFIG_ARM64_PTR_AUTH,
CONFIG_ARM64_PTR_AUTH_KERNEL, and CONFIG_BUILTIN_RETURN_ADDRESS_STRIPS_PAC.
Thus the pointer-stripping macros select the inline-assembly branch, while the
return-address builtin redefinition is disabled.  The frozen scope/symbol
records mark the operative macros and their conditional branches
PENDING_REVIEW; the ABI/lifetime records do not establish a Rust representation
for GNU statement-expressions, compiler builtins, fixed-register x30 binding,
or the C macro expansion contract used by the selected C consumers
(including arm64 stacktrace and process code).

No fresh Rust destination was written.  A Rust function or inline-assembly
wrapper would require assumptions about pointer/integer conversions,
register allocation and clobbers, compiler-builtin expansion, call-site
evaluation, and the C/Rust ABI that are not established by the pinned source
and frozen records.  Such a translation could change PAC stripping,
return-address semantics, or side effects.  The task is therefore BLOCKED
under the zero-difference rule pending an explicit source-backed ABI and
compiler-semantics decision.

No compiler, formatter, linker, test, emulator, debugger, or runtime command
was run.
