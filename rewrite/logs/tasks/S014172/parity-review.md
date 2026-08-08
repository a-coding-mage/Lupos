# Parity review — S014172 (slot 1)

Result: **FINDINGS**.  This review used the pinned
`vendor/linux/include/linux/kern_levels.h`, the candidate snapshot, and direct
printk consumer/parser context only.  No compiler, formatter, test, analyzer,
or historical Lupos source was used.

## P1 — `KERN_SOH`, `KERN_EMERG` through `KERN_CONT`: caller-side C token concatenation is absent

Linux defines these as object-like preprocessor macros.  At a use such as
`printk(KERN_INFO "enabled\\n")`, `include/linux/kern_levels.h:13` expands
before C parses adjacent string-literal tokens; the direct printk consumers in
`kernel/printk/printk.c:4208` and `include/linux/dev_printk.h:147-160` rely on
that property.  The candidate substitutes function-like `macro_rules!` macros
whose required invocation is `KERN_INFO!()` and whose own `concat!` only joins
the level’s two internal pieces.  It cannot appear in the source position of
the original object-like macro followed by an arbitrary caller literal.  Thus
the selected macros do not preserve their operative expansion contract.

## P2 — `KERN_SOH`, `KERN_DEFAULT`, and `KERN_CONT`: no source-proven C-string/NUL contract

Each Linux definition is a C string-literal token and thereby supplies a
trailing NUL when consumed as a `const char *`; `KERN_CONT` is parsed as the
`'c'` prefix by `kernel/printk/printk.c:2203-2205`.  The candidate supplies
Rust `&str` expressions (including `"\\u{0001}"`) and gives no pinned-source
ABI boundary or conversion preserving the terminating-NUL and pointer
contract.  The byte content alone is insufficient for calls that take the C
format-string representation.  Exact interoperation is therefore unresolved.

## P3 — `KERN_SOH_ASCII`: changed macro/token and C character-constant type contract

`include/linux/kern_levels.h:6` defines an object-like C character-constant
macro.  The candidate replaces it with `pub const ...: u8`.  C’s ordinary
character constant has `int` type and the macro may participate in caller
preprocessor/C expression typing; the fixed Rust `u8` item does not preserve
either the token-expansion or integer-type contract.  No frozen ABI or source
mapping establishes that this narrowing is exact for every selected use.

## P4 — `LOGLEVEL_SCHED` through `LOGLEVEL_DEBUG`: changed macro expansion/type behavior

The ten `LOGLEVEL_*` definitions at `include/linux/kern_levels.h:27-37` are
object-like integer macros, including negative values.  The candidate changes
them to typed `i32` items.  Although the numeric values match, the candidate
does not retain caller-side macro substitution or C integer-promotion behavior
in expressions.  No pinned selected caller/ABI mapping proves `i32` item
semantics are an exact replacement.

## P5 — all `KERN_*` macros: Rust macro visibility/import contract is not established

The header guard allows the C definitions to become available to every C
translation unit that includes this header.  The candidate’s `macro_rules!`
definitions are neither explicitly exported nor tied to the not-yet-generated
Rust module-index/import mechanism.  Its assertion that a path-local module
boundary replaces `__KERN_LEVELS_H__` provides no source-based mechanism by
which direct printk consumers obtain these names.  This is a missing selected
namespace/guard-equivalence mapping.

Disposition requested: do not accept as source-parity complete.  The missing
Rust representation of C token concatenation, C strings, and macro/type/guard
visibility must be established from frozen source/manifests or the task must
remain blocked rather than guessed.
