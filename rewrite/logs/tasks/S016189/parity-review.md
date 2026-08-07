# Parity review — S016189, attempt 1, slot 1

Result: **FINDINGS**. This was a manual source review only; no compiler,
formatter, linker, test, or runtime tool was used.

Reviewed source binding: `vendor/linux/include/uapi/linux/input-event-codes.h`
at `425f94c2954b1fe80ebdbf9b29854e89750355df` (1,016 lines) against
`src/include/uapi/linux/input-event-codes.rs` (1,024 lines). The sealed
proposal is `semantic-closure-proposal.tsv`, SHA-256
`62e1c3464bcd646dd6fcbf2147e496d4cc2246d2c4973923159a0466349a6121`,
with 3,189 COMPLETE records. Its scope row is
`SC1-322f4ed4b0d2eec28b0a86e8e7f15c30972b4c0049db89b4c3460ee31469864b`.

## PARITY-001 — C/UAPI preprocessor interface was replaced by Rust items

Linux symbols: the include guard `_UAPI_INPUT_EVENT_CODES_H` at lines 16–17,
and every one of the 795 value/alias `#define` symbols through
`SND_PROFILE_RING` at line 1014. The upstream header explicitly says at
lines 5–6 that it is included by C and devicetree source and therefore must
contain only comments and defines. Both selected architectures consume this
exact UAPI header: `rewrite/metadata/header_closure.tsv` records 261 AArch64
and 50 x86_64 consumers.

The candidate comments out the directives at lines 22 and 1022 and changes
all 796 definitions into `pub const` items (first value symbol at line 29,
last at line 1020). A Rust item is neither a C preprocessor replacement token
nor a definition visible to a devicetree source include. It cannot preserve
macro expansion, `#if`/`#ifdef` use, or the stated UAPI/DTS inclusion contract.
This is a mechanism and compatibility change, not a spelling-only port.

Closure mapping: the proposal marks all 3,188 per-architecture macro fields
COMPLETE (796 symbols × `selection_expression` and `status` × x86_64/AArch64).
Representative exact keys are `SC1-ac53fdc27c194fc239ca9cbcf07cabdf0a28a38100395c5b6bdb8156feca3315` /
`SC1-bd3a1645c3283a89b456f2b8d9e9233227d13de7e56280525f6a36d1ffe53589`
for AArch64 `INPUT_PROP_POINTER`,
`SC1-60384a163b487136d193d1c352a2d3e899b6ad9b75436946b32f2b6a237f5864` /
`SC1-85d1ef4c877256ec323d918a23c90246ca47320f129c4bb33d89205f81b2a3f9`
for x86_64 `INPUT_PROP_POINTER`, and
`SC1-8bf4ba32620f62271566991d432579069d994dae1ceed1e7a7903d64d32fbd64` /
`SC1-0e5b9c44647e1be2d5e0d61c5bcfdd778d8162ce91cc4caa1c541155146d47b5`
and `SC1-041b4815479d7b4b46177f50b990074716b3254ef0c4adfd655bec37867bc74f` /
`SC1-84c1731bcdb15a9e06b70ac4970a2f9c7bff983c85e4a8c8583b94bc743e1576`
for the final `SND_PROFILE_RING` mappings. Those COMPLETE fields are not
supported by the candidate's non-preprocessor representation.

## PARITY-002 — Every value macro has acquired the wrong signed type and promotion behavior

Linux symbols: all 795 non-guard macros, including `INPUT_PROP_POINTER`
(line 23), `KEY_RESERVED` (line 76), `KEY_MAX` (line 837), aliases such as
`KEY_HANGUEL` (line 200), and derived counters such as `INPUT_PROP_CNT`
(line 33) and `KEY_CNT` (line 838). Their replacement lists are unsuffixed
integer literals, aliases, or expressions over those literals. All literal
values in this header are at most `0x2ff`, hence are C `int` values on the
frozen x86_64 and AArch64 targets; aliases and the `*_CNT` expressions retain
the corresponding signed integer expression behavior after macro expansion.

The candidate gives each of the 796 declarations the explicit Rust type
`u32`, e.g. `INPUT_PROP_POINTER` at line 29, `KEY_RESERVED` at line 82,
`KEY_MAX` at line 841, and `SND_PROFILE_RING` at line 1020. This changes
signedness and the usual arithmetic/conversion behavior at every use site;
it is not equivalent to the C macro tokens. It also makes derived values
evaluate as Rust `u32` expressions rather than the source's expanded C `int`
expressions.

Closure mapping: the affected selected fields include
`SC1-b9bd2894f1d8d08b65c8b91ddb247142aaac8fbb26e29f83b36b683de588476a` /
`SC1-02d7cd388fe46ab8d393039e57e820935ff61230858ae0cc44f06dd065ea8cc9`
(AArch64 `KEY_RESERVED`),
`SC1-27f07f48b670125e258c1cd614b0f1f8e169ccc02f3dcef5e721743371258d68` /
`SC1-456bb5e5dd12134cdfb882bb8d9c7873ce7eff698fe98ca95d45fb54749e48b9`
(x86_64 `KEY_RESERVED`),
`SC1-1a940003cae8514080ddaa4a231e9b7c66afe7a188c17c1e7b7ccb29b1d966d1` /
`SC1-d7903b4ec7889c3d07793c6c4afc16f9884073cc329208138092d42aa96ff610`
(AArch64 `KEY_MAX`), and
`SC1-58c9ed8643ea95f4cbc23f40f03f84c7070e21bfcfc6b99c3c0360eba696df87` /
`SC1-8de34f553f5d6775a049143da2fbddb253b618a34b5414f0005a9fd200f89be5`
(x86_64 `KEY_MAX`). The same discrepancy applies to every per-architecture
macro field described in PARITY-001.

## PARITY-003 — The header guard has been changed into an exported value

Linux symbol: `_UAPI_INPUT_EVENT_CODES_H`. Linux uses `#ifndef` at line 16,
a blank replacement-list `#define` at line 17, and the matching `#endif` at
line 1016. This is inclusion state, not a numeric UAPI value.

The candidate has only comments for the conditional directives (lines 22 and
1022) and exports `pub const _UAPI_INPUT_EVENT_CODES_H: u32 = 1` at line 23.
It therefore adds an addressable/module-namespace value and fails to provide
the source guard's preprocessor-definedness and repeat-inclusion behavior.

Closure mapping: AArch64 guard condition keys
`SC1-f1deeb3a1432bd8f337688db39d2b5caff805010056ea5ea44fdad422034860e`
and `SC1-42b9c76c040953a889777cf6c433c918acca1414cdbc415e8231d3b3dee55e81`,
with macro keys
`SC1-ad49236e37be6803109f421ace115e9360f2dde28cd045f1a1541e2a580d30b4`
and `SC1-18c152c27227d42c2a24a2a84bf995c0183e845556fb851ae7269a4cbb3605f5`;
x86_64 counterparts
`SC1-c490e3822cf7b794b526b8937b082e1f852da368c3b0e64c6e10325b76bfec55`,
`SC1-5c955480914f7e021694996aadc1a4237aac2851906a18aa00227927626cbf52`,
`SC1-c6d5f1cd2d75d64ab63d2af3482c2d7ba280f8f89fe3353386db431d0efd1908`,
and `SC1-8eb6146ac527897d37af55f236126ac2fb88d3e51c7f9270a0875ad9b17c3235`
are incorrectly marked COMPLETE.

## Exhaustive mechanical comparison notes

- The source has 796 `#define` names (including the guard) and the candidate
  has 796 `pub const` names: no name is missing or extra.
- After removing whitespace and source comments, the 795 value/alias
  replacement expressions match textually; the guard alone differs because
  Linux has an empty replacement list while Rust assigns `1`.
- There are no configuration-selected value branches in this header beyond
  the include guard. The proposal contains 1,594 records for each approved
  architecture plus the common scope record, all marked COMPLETE.
- This header defines no C object layout, callable symbol, exported linkage,
  locking, allocation, refcount, RCU, or error path. No additional such
  discrepancy was found by source inspection.
- The candidate retains the upstream SPDX/copyright text and no unauthorized
  Lupos branding was observed; `rewrite/BRANDING_ALLOWLIST.tsv` has no entry
  for this header.
