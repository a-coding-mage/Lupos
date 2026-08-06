# Parity review: S014648 (slot 1)

## Finding P1 — reject: string-literal macro semantics were narrowed to pointer constants

**Severity:** blocking

All four upstream definitions at
`vendor/linux/include/linux/pinctrl/pinctrl-state.h:33-36` are object-like
macros whose replacement lists are C string literals.  Every expansion is a
NUL-terminated character-array expression, not a `const char *` declaration.
In particular, the C preprocessor permits the expanded literal to participate
in adjacent-literal concatenation, and the resulting expression retains its
array extent before ordinary array-to-pointer decay.  The candidate instead
exposes `PINCTRL_STATE_{DEFAULT,INIT,IDLE,SLEEP}` as `*const c_char` constants
at `src/include/linux/pinctrl/pinctrl-state.rs:18,23,28,34`.

This is already required by selected pinned source:
`vendor/linux/drivers/i2c/i2c-core-base.c:327` uses
`PINCTRL_STATE_DEFAULT " state not found for GPIO recovery\\n"`.  The upstream
macro expands there to one string literal (`"default state not found for GPIO
recovery\\n"` including its terminating NUL).  A raw pointer constant cannot
be token-concatenated with the following literal and therefore cannot preserve
that call site's source/array semantics.  The pointer form also loses valid
literal-array operations such as `sizeof(PINCTRL_STATE_DEFAULT)`, indexing,
and array initialization, while making every use share the one named backing
object rather than representing the macro's literal expansion.

The private byte arrays do contain the correct ASCII bytes and terminating
NULs, but that does not repair the public macro representation or the affected
provenance/expansion behavior.

**Required resolution:** Do not close this task with pointer constants as a
translation of the C macros.  Establish and apply a project-wide, call-site
preserving representation for C string-literal macros (including adjacent
literal concatenation and array extent), or block/escalate if the fresh Rust
tree cannot express that contract without changing the dependent translated
call sites.  Record the final semantic decision in the task's required
manifests/resolution.

## Coverage

Reviewed the complete 38-line pinned header, all four selected operative
macros for both frozen architectures, the complete candidate, and pinned
in-tree uses of the four macro names.  No source files other than this report
were changed; no build, test, format, or runtime command was run.
