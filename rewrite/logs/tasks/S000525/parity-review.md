# Parity review — S000525 (slot 1)

Reviewed `src/arch/x86/include/asm/extable_fixup_types.rs` against pinned
`vendor/linux/arch/x86/include/asm/extable_fixup_types.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the selected x86_64 frozen
configuration, `arch/x86/include/asm/asm.h`, `arch/x86/include/asm/extable.h`,
and `arch/x86/mm/extable.c`.

## Finding P0-1 — upstream SPDX identifier was changed

`extable_fixup_types.h:1` is exactly `/* SPDX-License-Identifier: GPL-2.0 */`.
The candidate begins `// SPDX-License-Identifier: GPL-2.0-only`.  These are
not the same SPDX identifier, and the rewrite protocol requires retention of
the upstream SPDX identifier; branding is not an allowlisted license change.

Required resolution: restore the source header to `GPL-2.0` exactly.

## Verified parity items

- The four masks retain the source values and C literal-width behavior:
  `TYPE=0x000000ff`, `REG=0x00000f00`, `FLAG=0x0000f000`, and unsigned-int
  `IMM=0xffff0000`.  `exception_table_entry.data` is `int` in
  `arch/x86/include/asm/extable.h:23-25`; the unsigned immediate mask is
  specifically consumed by `FIELD_GET_SIGNED()` in `arch/x86/mm/extable.c:323-325`.
- Shift positions are exactly `8`, `12`, and `16`.  The derived segment fields
  are `0x800`, `0x900`, `0xa00`, and `0xb00`; clear-AX, clear-DX, and combined
  flags are `0x1000`, `0x2000`, and `0x3000`.
- `EX_DATA_REG`, `EX_DATA_FLAG`, and `EX_DATA_IMM` preserve the 32-bit encoded
  result for all source-derived values.  In particular,
  `EX_TYPE_EFAULT_REG` is `17 | ((-14) << 16)`, whose stored 32-bit pattern is
  `0xfff20011`; this is the immediate value decoded by the `IMM_REG` path in
  `arch/x86/mm/extable.c:356-364`.
- Every defined `EX_TYPE_*` value is present with its pinned numerical or
  composed encoding: the intentional hole at type 4 remains absent, and values
  0 through 21 otherwise match the header, including POP, IMM_REG, UCOPY, and
  ERETU encodings.
- The selected configuration has `CONFIG_X86_64=y`, `CONFIG_X86_MCE=y`,
  `CONFIG_BPF=y`, and `CONFIG_X86_FRED` unset.  The header has no conditional
  definitions, so all its type constants, including `EX_TYPE_BPF` and
  `EX_TYPE_ERETU`, must remain available; the candidate does so.
- No candidate-side altered register encoding, flag, type selector, immediate
  field placement, or error-value encoding was found beyond P0-1.

Verdict: reject pending P0-1 correction.  No build, compiler, formatter, test,
or runtime command was run.
