# Rust semantic review — S016028 / P02 / attempt 1 / slot 2

Reviewed independently against `vendor/linux/include/uapi/asm-generic/termbits-common.h` and direct UAPI consumers in `vendor/linux/include/uapi/asm-generic/termbits.h` and `vendor/linux/drivers/tty/tty_baudrate.c`. No compiler, formatter, analyzer, test, or runtime tool was used. No source was edited.

## Finding P1 — fixed Rust constant types do not preserve the C macro expression contract

`termbits-common.h` defines preprocessor macros, not typed objects. The unsuffixed literals through `CMSPAR` have C `int` expression type; `CRTSCTS` is an `unsigned int` expression because `0x80000000` does not fit in `int` on the approved ABI. At use sites C's usual arithmetic conversions contextualize those expressions to `tcflag_t`/`unsigned int`: for example `tty_baudrate.c` combines `CBAUD`, `B0`, and `IBSHIFT` with `termios->c_cflag`, while `termbits.h` uses the typedefs in UAPI structures.

The candidate instead exports most flags as `i32`, but exports `CRTSCTS` as `u32`. Rust has no usual arithmetic conversion for `i32` and `u32`, so the translated identifiers cannot participate in the corresponding bitwise expressions without call-site casts or a separate abstraction. Those casts/abstraction are not present in this file and would select a behavior/type contract absent from the C macro. Assigning `i32` to the flags also makes `~FLAG` and shifts operate under Rust's signed semantics rather than the original context's converted unsigned width. The numeric bit patterns alone therefore do not establish the UAPI macro contract.

This is a source-level blocker for the asserted `SOURCE_REVIEWED_VALUE` closure entries: the pinned evidence records macro text but provides no exact Rust representation that retains both preprocessing and C contextual integer conversion behavior. The proposal must not be sealed as complete until the project establishes that representation and all consumers use it coherently.

Affected exact semantic-closure keys:

- `SC1-1215336c9dc43faae4f57a35b8fe9aa8c43214a6b09a17008c169ab71205b3e3` (`IGNBRK`, aarch64, `selection_expression`)
- `SC1-218033217a103ed09f4b0d4096ee376aaa13ec41214770018bdb8fce581ed16a` (`IGNBRK`, x86_64, `selection_expression`)
- `SC1-693578035391003508b49fe80c604c00113df5b9116d1e2b3124185cc3d8ad05` (`ADDRB`, aarch64, `selection_expression`)
- `SC1-452657116f700fab51b96eba7f2bd341f6176d8fd0ddd036da38e0398a014b26` (`ADDRB`, x86_64, `selection_expression`)
- `SC1-cb24f30c80bd60bdc7f555900e71990cbbb001672272dcc86acda5ad656bd7f7` (`CMSPAR`, aarch64, `selection_expression`)
- `SC1-3c9b05c0a1c6ef4f4dc65072869891a31a86c7ec37a4f67c916481197e253897` (`CMSPAR`, x86_64, `selection_expression`)
- `SC1-65f30fe75b7667b6ceb2dd112cb7854bafc4e7f00090c41e09948f25781a4dfa` (`CRTSCTS`, aarch64, `selection_expression`)
- `SC1-77eab523a45e60727fd8fd05b373104e719edee3a9befa4824d307b9b244a380` (`CRTSCTS`, x86_64, `selection_expression`)
- `SC1-37e3907296ea5b844d623c5f166087f9222dfe98e65a4a4404efbc6acc3c93d1` (`IBSHIFT`, aarch64, `selection_expression`)
- `SC1-e169e0f7114fc0e8852869f7d90b97a8db4deaae65a517c38505f4cd440e0a9e` (`IBSHIFT`, x86_64, `selection_expression`)

No ownership, pointer, aliasing, layout, allocation, Drop, or `unsafe` concern is otherwise present: this header contains only scalar aliases and macro definitions.
