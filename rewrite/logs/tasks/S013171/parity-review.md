# S013171 parity review — slot 1

Reviewer: `parity_p01_s013171` (`terra`, `high`)

Scope reviewed: pinned `vendor/linux/include/dt-bindings/leds/common.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, current
`src/include/dt-bindings/leds/common.rs`, the S013171 candidate snapshot, and
the exact S013171 frozen rows. The header is self-contained: it has no direct
pinned header dependencies. No compiler, formatter, linker, test,
rust-analyzer diagnostic, or runtime command was invoked.

Frozen inputs: Phase-0 identity
`0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`; queue
`cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`; scope
`b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a`;
symbols `7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`;
ABI `ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`;
lifetimes `0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`.

FINDINGS

--finding Linux symbol `__DT_BINDINGS_LEDS_H`: the selected C conditional and
guard macro are absent. The pinned header has `#ifndef __DT_BINDINGS_LEDS_H`
at line 12, `#define __DT_BINDINGS_LEDS_H` at line 13, and the matching
`#endif` at line 114; the frozen symbol rows select `ifndef@12`,
`__DT_BINDINGS_LEDS_H`, and `endif@114` for both architectures. The candidate
has no corresponding conditional or guard identifier. Repeated C inclusion
therefore has its upstream preprocessing behavior and identifier unavailable,
rather than the selected guard behavior being mapped or explicitly preserved.

--finding Linux symbols `LEDS_TRIG_TYPE_EDGE`, `LEDS_TRIG_TYPE_LEVEL`,
`LEDS_BOOST_OFF`, `LEDS_BOOST_ADAPTIVE`, `LEDS_BOOST_FIXED`,
`LED_COLOR_ID_WHITE`, `LED_COLOR_ID_RED`, `LED_COLOR_ID_GREEN`,
`LED_COLOR_ID_BLUE`, `LED_COLOR_ID_AMBER`, `LED_COLOR_ID_VIOLET`,
`LED_COLOR_ID_YELLOW`, `LED_COLOR_ID_IR`, `LED_COLOR_ID_MULTI`,
`LED_COLOR_ID_RGB`, `LED_COLOR_ID_PURPLE`, `LED_COLOR_ID_ORANGE`,
`LED_COLOR_ID_PINK`, `LED_COLOR_ID_CYAN`, `LED_COLOR_ID_LIME`, and
`LED_COLOR_ID_MAX`: upstream lines 16–41 define each as an unsuffixed decimal
integer macro. On the frozen 64-bit Linux targets those integer literal
expressions have C `int` type and retain ordinary C integer-promotion and
signed-operation behavior at every macro expansion. Candidate lines 9–33
instead define fixed `u32` constants. This changes the selected identifiers'
expression type, signedness, promotion, comparison, negation, and overflow
semantics; equal literal values alone do not preserve the macro contract.

--finding Linux symbols `LED_FUNCTION_CAPSLOCK`, `LED_FUNCTION_SCROLLLOCK`,
`LED_FUNCTION_NUMLOCK`, `LED_FUNCTION_FNLOCK`, `LED_FUNCTION_KBD_BACKLIGHT`,
`LED_FUNCTION_POWER`, `LED_FUNCTION_DISK`, `LED_FUNCTION_CHARGING`,
`LED_FUNCTION_STATUS`, `LED_FUNCTION_MICMUTE`, `LED_FUNCTION_MUTE`,
`LED_FUNCTION_PLAYER1`, `LED_FUNCTION_PLAYER2`, `LED_FUNCTION_PLAYER3`,
`LED_FUNCTION_PLAYER4`, `LED_FUNCTION_PLAYER5`, `LED_FUNCTION_ACTIVITY`,
`LED_FUNCTION_ALARM`, `LED_FUNCTION_BACKLIGHT`, `LED_FUNCTION_BLUETOOTH`,
`LED_FUNCTION_BOOT`, `LED_FUNCTION_CPU`, `LED_FUNCTION_DEBUG`,
`LED_FUNCTION_DISK_ACTIVITY`, `LED_FUNCTION_DISK_ERR`,
`LED_FUNCTION_DISK_READ`, `LED_FUNCTION_DISK_WRITE`, `LED_FUNCTION_FAULT`,
`LED_FUNCTION_FLASH`, `LED_FUNCTION_HEARTBEAT`, `LED_FUNCTION_INDICATOR`,
`LED_FUNCTION_LAN`, `LED_FUNCTION_MAIL`, `LED_FUNCTION_MOBILE`,
`LED_FUNCTION_MTD`, `LED_FUNCTION_PANIC`, `LED_FUNCTION_PROGRAMMING`,
`LED_FUNCTION_RX`, `LED_FUNCTION_SD`, `LED_FUNCTION_SPEED_LAN`,
`LED_FUNCTION_SPEED_WAN`, `LED_FUNCTION_STANDBY`, `LED_FUNCTION_TORCH`,
`LED_FUNCTION_TX`, `LED_FUNCTION_USB`, `LED_FUNCTION_WAN`,
`LED_FUNCTION_WAN_ONLINE`, `LED_FUNCTION_WLAN`, `LED_FUNCTION_WLAN_2GHZ`,
`LED_FUNCTION_WLAN_5GHZ`, `LED_FUNCTION_WLAN_6GHZ`, and `LED_FUNCTION_WPS`:
the pinned macros at lines 46–112 expand to C string-literal expressions,
whose arrays include a terminating NUL and whose C array/pointer and `sizeof`
semantics apply at each expansion. Candidate lines 36–84 replace every one
with a Rust `&str`, a UTF-8 slice with a length and no C-string/NUL contract.
This changes representation, FFI suitability, length/`sizeof` behavior, and
macro expression semantics despite retaining each visible text literal.

--finding Linux symbol `__DT_BINDINGS_LEDS_H` (file-level source contract):
the pinned header's SPDX identifier at line 1 is
`SPDX-License-Identifier: (GPL-2.0 OR BSD-2-Clause)`, while the candidate's
provenance line is `SPDX-License-Identifier: GPL-2.0-only`. This removes the
upstream BSD-2-Clause alternative and does not retain the exact upstream SPDX
identifier required for the selected guarded header.

SC1 record keys: none supplied by the current candidate proposal; none
invented.

