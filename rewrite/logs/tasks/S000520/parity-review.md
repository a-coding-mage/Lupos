# Parity review — S000520 (slot 1)

## Scope and evidence

- Queue row: `REVIEWING`; source `arch/x86/include/asm/emulate_prefix.h`; destination `src/arch/x86/include/asm/emulate_prefix.rs`; architecture `x86_64`.
- Pinned revision verified: `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Compared the complete 14-line pinned header with the candidate and inspected the frozen x86_64 configuration plus all pinned macro consumers.  No compiler, formatter, rust-analyzer, build, test, debugger, or runtime tool was used.

## Findings

### P1 — C token-list macro semantics were replaced with typed Rust array values

Linux symbols `__XEN_EMULATE_PREFIX` and `__KVM_EMULATE_PREFIX` (header lines 11–12) are object-like preprocessor macros.  Each expands to five untyped C integer tokens separated by commas, so the surrounding consumer supplies both the contextual type and syntax.  Pinned consumers demonstrate that contract:

- `arch/x86/lib/insn.c:82-83` inserts each expansion into a C array initializer whose element type is `insn_byte_t`.
- `arch/x86/kvm/x86.c:8024` inserts the KVM expansion into a `char` array initializer.
- `arch/x86/include/asm/xen/interface.h:387` incorporates the Xen expansion into an assembler `.byte` form.

The candidate instead declares `pub const ...: [u8; 5]`.  That is one typed Rust array expression, not a comma-token expansion: it cannot occupy the above initializer/assembler positions and fixes an element type that Linux deliberately leaves to the use site.  The selected header has no Kconfig conditional around either macro; `CONFIG_XEN` being unset and `CONFIG_KVM_GUEST=y` in the frozen configuration do not remove this macro-level contract.  This is a semantic/interface mismatch, not a cosmetic macro-to-constant conversion.

Required resolution: preserve each macro's use-site expansion semantics (including its permitted initializer and assembler contexts), or provide a frozen, source-backed translation mechanism and update every selected consumer consistently.  A bare `[u8; 5]` constant cannot be accepted as the complete translation of these operative macros.

## Checked without finding

- Byte values and ordering match exactly: Xen `0x0f, 0x0b, 0x78, 0x65, 0x6e`; KVM `0x0f, 0x0b, 0x6b, 0x76, 0x6d` (UD2 followed by the respective ASCII signature).
- Candidate immutable provenance identifies the correct Linux source, pinned revision, `x86_64` architecture, and task ID.  No branding deviation is present.
- The Linux include guard is C-preprocessor-only and does not itself create an additional Rust runtime/interface requirement.

## Verdict

Reject pending resolution of P1.  The candidate preserves the ten byte values but not the two operative macro expansion/type contracts.
