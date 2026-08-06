# Resolution — S014648

Reopened the complete pinned
`vendor/linux/include/linux/pinctrl/pinctrl-state.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the task's frozen x86_64 and
AArch64 header-closure records, the final candidate, both independent review
reports, and the selected `drivers/i2c/i2c-core-base.c:323-329` consumer. No
compiler, formatter, linker, test, runtime, or build command was run.

## P1 / R1 — accepted and fixed

Upstream lines 33--36 are object-like macros whose replacement lists are C
string-literal arrays, not declarations of `const char *` objects. The final
source therefore exposes `PINCTRL_STATE_DEFAULT`, `PINCTRL_STATE_INIT`,
`PINCTRL_STATE_IDLE`, and `PINCTRL_STATE_SLEEP` as public NUL-terminated
`[c_char; 8]`, `[c_char; 5]`, `[c_char; 5]`, and `[c_char; 6]` value arrays.
The exact ASCII bytes and one trailing NUL are retained. There is no named
backing object, exported symbol, raw pointer constant, or pointer-decay
substitute.

The selected i2c source proves that this distinction is operative:
`PINCTRL_STATE_DEFAULT " state not found for GPIO recovery\\n"` is one C
literal after preprocessing. Rust has no adjacent-literal token syntax, so a
translated consumer must lower that *complete expanded C literal* as one
NUL-terminated value array at its own use. It must not concatenate pointers or
replace the header macro with a pointer. This header's array values preserve
the standalone literal-array extent, indexing, and aggregate-initializer
surface; the source macro itself does not declare a named ABI object.

## Semantic-record closure

All fourteen S014648 `SYMBOLS.tsv` rows are now `COMPLETE`: the three include
guard records per architecture record their no-Rust-item treatment, and the
four literal macros per architecture record their exact value-array mappings
and the adjacent-literal consumer evidence. `rewrite/SCOPE.tsv` likewise
records S014648 semantic status as `COMPLETE`.

`ABI.tsv`, `LIFETIMES.tsv`, `DRIVER_ABI.tsv`, and `BLOCKERS.tsv` contain no
S014648 rows. That is correct for this declaration-only, non-driver header:
it declares no layout, linkage, ownership, allocation, cleanup, locking, RCU,
refcount, callback, or retained-pointer contract. No task-local semantic
record remains pending.

All five required task evidence files exist. The destination retains the exact
upstream GPL-2.0 SPDX identifier and immutable source/revision/architecture/
task provenance, and introduces no test, placeholder, unsafe code, driver
port, or branding change.
