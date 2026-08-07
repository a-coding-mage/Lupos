# Rust review — S016189

Result: REJECT

Reviewed independently against `vendor/linux/include/uapi/linux/input-event-codes.h` at pinned revision `425f94c2954b1fe80ebdbf9b29854e89750355df` and `src/include/uapi/linux/input-event-codes.rs`. The queue row was `REVIEWING` for P01/S016189. No compiler, formatter, test, rust-analyzer diagnostic, or historical Rust source was used.

1. **Three `pub const` declarations have no terminating semicolon outside their multiline block comment.**
   - Upstream `KEY_SWITCHVIDEOMODE` is the macro at `vendor/linux/include/uapi/linux/input-event-codes.h:307-308`. Candidate `src/include/uapi/linux/input-event-codes.rs:313-314` has `227 /* ... video; ... */`; the only semicolon is inside the comment, so the item has no Rust terminator.
   - Upstream `KEY_BRIGHTNESS_AUTO` is at `vendor/linux/include/uapi/linux/input-event-codes.h:330-332`. Candidate `src/include/uapi/linux/input-event-codes.rs:336-338` likewise puts the only semicolon inside the opening comment text and has none after `*/`.
   - Upstream `SW_RFKILL_ALL` is at `vendor/linux/include/uapi/linux/input-event-codes.h:938-939`. Candidate `src/include/uapi/linux/input-event-codes.rs:944-945` repeats the same error.
   - This is established by manual syntax inspection: after each closing `*/`, the next token is a new `pub const`, rather than the required `;`. It rejects the candidate independently of any later build.

Source-review checks completed:

- The source inventory has 796 operative macros per selected architecture: 795 value/alias macros plus the C include guard. The candidate contains all 795 non-guard names. The C header explicitly records that it is consumed by C and devicetree and therefore contains only comments and defines (`input-event-codes.h:4-7`); the candidate is a Rust-module projection, not a replacement C preprocessor header.
- Excluding the three malformed multiline-comment declarations above, every one-line macro name, numeric expression, and alias expression matches the pinned source. Representative aliases retained include `KEY_HANGUEL`, `KEY_SCREENLOCK`, `BTN_A`, `KEY_BRIGHTNESS_ZERO`, and `SW_RADIO`.
- All projected constants are `i32`. Every upstream numeric value in this header is non-negative and at most `KEY_MAX = 0x2ff` (`input-event-codes.h:848`), so no signed range, truncation, sign-extension, shift, overflow, pointer, endian, or layout discrepancy is present in the values themselves. The original literals are ordinary C integer expressions; callers needing other integer types must preserve their explicit source-level conversion at each use.
- This macro-only file declares no ownership-bearing data, references, pointers, `unsafe`, FFI, `repr`, pinning, interior mutability, callbacks, allocation, `Drop`, atomics, or `Send`/`Sync` boundary. No panic/placeholder/test token is present. No branding difference was found.

Required resolution: restore one terminating semicolon immediately after each of the three closing multiline comments, then rerun both independent source reviews after the source change under the Phase 1 protocol.
