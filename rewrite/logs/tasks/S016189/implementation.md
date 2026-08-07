# S016189 implementation

Task/source identity was verified before mutation:

- Queue row: `S016189	src/include/uapi/linux/input-event-codes.rs	2026-08-07T11:57:24.116Z	2026-08-07T12:27:53.342Z		IN_PROGRESS	include/uapi/linux/input-event-codes.h	common	include	101.6	low		luna	P01	1	codex-root-repair-20260807-p01	2026-08-07T13:57:53.342Z`
- Branch: `feat/bun-like-rewrite-test`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df` (vendor/linux.SHA)
- Scope: `rewrite/SCOPE.tsv:16190`; class `RUST_TRANSLATE`; architectures `common`; Kconfig evidence is the frozen x86_64 and AArch64 configurations and header-closure metadata.
- Destination: `src/include/uapi/linux/input-event-codes.rs` (fresh file).

## Source and selection evidence

The complete pinned header `vendor/linux/include/uapi/linux/input-event-codes.h` (1016 lines) was read. It contains comments, the include guard at lines 16--17 and 1016, and 796 `#define` directives. `rewrite/SYMBOLS.tsv` contains 1,596 selected rows for S016189 (all 798 source directives, including guard, for each of aarch64 and x86_64). There are no configuration conditionals inside the header other than the guard; the scope row records common architecture selection and the two frozen configuration/header-closure consumers.

## Translation decisions (final)

1. The source SPDX notice is retained exactly as `GPL-2.0-only WITH Linux-syscall-note`. Immutable provenance records the pinned path, SHA, `common` architecture union, and task ID.
2. C comments and copyright/license text remain comments. The C include guard is represented by the exported `pub const _UAPI_INPUT_EVENT_CODES_H: u32 = 1` and adjacent guard comments, preserving the one-definition intent without a C preprocessor.
3. Every source `#define NAME VALUE` is represented in source order as `pub const NAME: u32 = VALUE;`. Numeric literals retain spelling and width-independent non-negative value. Alias definitions retain symbolic RHS (for example `KEY_HANGUEL = KEY_HANGEUL`, `BTN_A = BTN_SOUTH`, and `SW_RADIO = SW_RFKILL_ALL`), preserving dependency and promotion behavior.
4. Computed count definitions retain their source expressions exactly: `INPUT_PROP_CNT = (INPUT_PROP_MAX + 1)`, `EV_CNT = (EV_MAX+1)`, `SYN_CNT = (SYN_MAX+1)`, `KEY_CNT = (KEY_MAX+1)`, `REL_CNT = (REL_MAX+1)`, `ABS_CNT = (ABS_MAX+1)`, `SW_CNT = (SW_MAX+1)`, `MSC_CNT = (MSC_MAX+1)`, `LED_CNT = (LED_MAX+1)`, `REP_CNT = (REP_MAX+1)`, and `SND_CNT = (SND_MAX+1)`.
5. The two continued source comments (the `KEY_SWITCHVIDEOMODE` description and `SW_RFKILL_ALL` description) are retained as Rust comments on continuation lines; they do not alter values or ordering. No symbols, branches, tests, drivers, module indexes, or other task paths were edited.

## Final semantic proposal

All 796 directives selected in each architecture row map one-to-one to constants in original order. The final file has 1,024 lines and 796 constants; no `todo!`, `unimplemented!`, test configuration, placeholder, or compiler-derived change is present. UAPI names, numeric values, aliases, count expressions, comments, and guard identity are unchanged except for the required C-to-Rust declaration syntax.

Destination SHA-256 after sealing: `de3aa54b04af8f418244bb208ab99eeb51c73f8ae91de80b83e446ae6a94a90c`.
