# Resolution S016189

Pinned source reopened: `vendor/linux/include/uapi/linux/input-event-codes.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df`. The frozen queue verifies
against `9fb31be9d78d9923c9541c26a34efe1502a66e5d0cca06dbd4776a756592cdc9`;
S016189 is the `APPLYING` P01 row for this exact source and destination.

## Parity finding 1 — KEY_SWITCHVIDEOMODE / KEY_KBDILLUMTOGGLE

Accepted and corrected. Upstream lines 307-309 define independent macros
`KEY_SWITCHVIDEOMODE` as `227` and `KEY_KBDILLUMTOGGLE` as `228`. The Rust
block comment after the former now ends before an explicit item terminator;
the candidate therefore contains two separate constants with those values.

## Parity finding 2 — KEY_BRIGHTNESS_AUTO / KEY_BRIGHTNESS_ZERO

Accepted and corrected. Upstream lines 330-333 define
`KEY_BRIGHTNESS_AUTO` as `244` and `KEY_BRIGHTNESS_ZERO` as an alias of it.
The candidate now has the missing terminator after the multiline comment, so
the following `KEY_BRIGHTNESS_ZERO: i32 = KEY_BRIGHTNESS_AUTO` item is a
separate alias exactly as in the source.

## Parity finding 3 — SW_RFKILL_ALL / SW_RADIO

Accepted and corrected. Upstream lines 938-940 define `SW_RFKILL_ALL` as
`0x03` and `SW_RADIO` as its alias. The candidate now terminates
`SW_RFKILL_ALL` immediately after its multiline comment, leaving `SW_RADIO`
as the separate alias item required by the source.

## Parity finding 4 — C/devicetree preprocessor interface

Rejected as a defect in this Rust mapping, with the C/devicetree contract
retained by the pinned Linux header rather than silently substituted. The
source itself states at lines 6-7 that it is included by C and devicetree
source, and lines 16-17 and 1016 retain its C include guard. That exact
header remains under `vendor/linux/include/uapi/linux/` unchanged. The frozen
`FILE_MAP.tsv` records its selected C consumers with compile commands whose
include paths use `vendor/linux/include` and `vendor/linux/include/uapi`;
`vendor/linux/include/uapi/linux/input.h:20` includes this header, while the
selected `vendor/linux/net/rfkill/input.c:13` includes `<linux/input.h>`.

The frozen S016189 scope row independently requires the path-preserving Rust
projection `src/include/uapi/linux/input-event-codes.rs`. Its `pub const`
items are therefore the Rust representation for rewritten Rust consumers;
they are not a replacement file at the C or devicetree include path. The
source macro definitions and repeated-include guard remain available to every
original C/devicetree consumer from the unmodified pinned header. Adding a
second C preprocessor bridge or changing the vendor header would exceed this
frozen task's destination scope and would not improve that already-preserved
interface.

## Rust finding 1 — missing item terminators

Accepted and corrected. This finding covers the same three malformed
declarations identified above. Each closing multiline comment for
`KEY_SWITCHVIDEOMODE`, `KEY_BRIGHTNESS_AUTO`, and `SW_RFKILL_ALL` now has the
required following `;`; their paired source constants
`KEY_KBDILLUMTOGGLE`, `KEY_BRIGHTNESS_ZERO`, and `SW_RADIO` remain independent
items and preserve the upstream literal/alias relationship.

## Semantic-record closure

The complete pinned header contains one C include guard and 795 non-guard
object-like value/alias macros, with no configuration-dependent branch other
than that guard. The frozen symbol inventory records the same macro set for
both approved architectures. Every numeric literal and every `MAX + 1`
expression is an ordinary C integer constant expression no greater than
`KEY_MAX = 0x2ff` (upstream line 848); each unsuffixed literal therefore has
type `int` before the source's explicit conversions at use sites. The Rust
`i32` constants preserve those `int` values and the ordered alias expressions
without truncation, sign change, overflow, side effects, layout, ownership,
locking, lifetime, or ABI state. The C guard is intentionally represented by
the retained vendor header, not as a Rust item.

The destination contains exactly one immutable occurrence of each required
provenance field: source path, pinned revision, architecture set, and task ID.
No source behavior was weakened, no placeholder or test was introduced, and
no compiler, formatter, linker, runtime, or diagnostic tool was used.
