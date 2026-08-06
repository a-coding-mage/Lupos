# Parity review — S016476

Reviewed task `S016476` on `feat/bun-like-rewrite-test`, against pinned
`vendor/linux` revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
Manual source inspection only; no compiler, formatter, test, or analyzer was
run.

## Result: REJECT — one parity finding

### P1 — flag macro expression types were changed to `u32`

`vendor/linux/include/uapi/linux/wait.h:5-14` defines `WNOHANG`,
`WUNTRACED`, `WSTOPPED`, `WEXITED`, `WCONTINUED`, `WNOWAIT`,
`__WNOTHREAD`, `__WALL`, and `__WCLONE` as object-like C macros.  Under the
selected 32-bit `int` ABI, the unsuffixed hexadecimal literals for every one
except `__WCLONE` have type `int`; `WSTOPPED` expands to the `int` expression
`WUNTRACED`; only `0x80000000` in `__WCLONE` has type `unsigned int`.

The candidate at `src/include/uapi/linux/wait.rs:8-25` instead declares all
nine names as `u32`.  This loses the source macros' individual expression
types and their ordinary C contextual conversions.  The selected consumers
demonstrate that this is operative: `kernel/exit.c:1769-1841` and
`1880-1911` use an `int options`/`int wo_flags` context with these masks, and
`kernel/pid_namespace.c:244` passes `__WALL` to the `int options` argument of
`kernel_wait4`.  In particular, C promotes/converts `int options` with
`__WCLONE` to `unsigned int` for the bitwise operation, while the other masks
remain `int`; the candidate forces an unsigned type for every mask and cannot
be used with an `i32` translation of these selected parameters without
additional, semantically material conversion policy.

Resolve by preserving the source literal/expression type behavior and by
documenting the required bit-pattern/conversion handling at the Rust boundary
for the selected signed `options` consumers.  Do not treat all flag macros as
one uniform `u32` API.

## Exhaustive comparison record

| Source macro | Source expansion/type | Candidate | Finding status |
| --- | --- | --- | --- |
| `WNOHANG` | `0x00000001`, `int` | `u32`, same bits | P1 |
| `WUNTRACED` | `0x00000002`, `int` | `u32`, same bits | P1 |
| `WSTOPPED` | `WUNTRACED`, `int` composite alias | `u32 = WUNTRACED` | P1 |
| `WEXITED` | `0x00000004`, `int` | `u32`, same bits | P1 |
| `WCONTINUED` | `0x00000008`, `int` | `u32`, same bits | P1 |
| `WNOWAIT` | `0x01000000`, `int` | `u32`, same bits | P1 |
| `__WNOTHREAD` | `0x20000000`, `int` | `u32`, same bits | P1 |
| `__WALL` | `0x40000000`, `int` | `u32`, same bits | P1 |
| `__WCLONE` | `0x80000000`, `unsigned int` | `u32`, same bits | covered by P1; its unsigned source type is distinct |
| `P_ALL` | `0`, `int` | `i32 = 0` | pass |
| `P_PID` | `1`, `int` | `i32 = 1` | pass |
| `P_PGID` | `2`, `int` | `i32 = 2` | pass |
| `P_PIDFD` | `3`, `int` | `i32 = 3` | pass |

All thirteen macro spellings and numeric bit patterns are present.  `WSTOPPED`
remains an alias rather than a duplicated literal.  The source header has no
configuration-dependent macro branch (only `_UAPI_LINUX_WAIT_H` include
guard); Phase 0 selects it as `common` for both frozen architectures.  Header
closure records the selected direct consumers as `kernel/exit.c` and
`kernel/pid_namespace.c` on both architectures.  The candidate has exact
SPDX text and valid immutable provenance (source path, frozen revision,
architectures, task ID).  No matching S016476 row exists in the frozen ABI or
lifetime manifests; therefore this unresolved type/conversion contract must
be closed during application rather than inferred from those manifests.
