# Implementation record: S016476

- Pipeline: `P01`
- Role/model/effort: isolated fallback implementer / Terra / medium
- Linux source: `include/uapi/linux/wait.h`
- Destination: `src/include/uapi/linux/wait.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (`x86_64`, `aarch64`)

## Frozen-input verification

Verified the required branch, queue fingerprint (`af93adda6e7845ec178dc63a9462f88384392f78353165ea5a583ef78fcf423c`), P01 lease for S016476, and Phase 0 identity. The identity binds the pinned Linux revision above and both frozen configurations. The task is selected for both architectures through `kernel/exit.o`; the Phase 0 header closure also identifies `kernel/pid_namespace.o` as the other consumer.

## Source mapping

The complete pinned header contains its include guard, nine wait-option macros, and four `waitid` selector macros; it has no types, layouts, functions, or configuration branches. Rust modules make the C include guard unnecessary. The option macros are represented as `u32` constants so every specified 32-bit bit pattern, including `__WCLONE = 0x80000000`, remains representable. `WSTOPPED` retains its exact alias relationship to `WUNTRACED`. The `P_*` selectors are represented as `i32` constants, matching their C `int`-valued use in `kernel_waitid_prepare`.

The direct selected consumers preserve the upstream interpretation: `kernel/exit.c` validates and tests the option masks and dispatches on `P_ALL` through `P_PIDFD`; `kernel/pid_namespace.c` passes `__WALL` to `kernel_wait4`. No source behavior, ABI layout, ownership, or lifetime is introduced by this constants-only UAPI header.

## Phase 1 constraints

No compiler, formatter, linker, test, runtime tool, or rust-analyzer diagnostic was invoked. No historical Lupos Rust source was inspected. No tests, stubs, module indexes, or files outside this task's destination and evidence record were added.
