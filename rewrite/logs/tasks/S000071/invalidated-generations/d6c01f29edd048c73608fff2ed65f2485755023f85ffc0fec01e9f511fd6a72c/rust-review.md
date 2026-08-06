# Rust review — S000071

Reviewed `vendor/linux/arch/arm64/include/asm/gpr-num.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df` and the candidate
`src/arch/arm64/include/asm/gpr-num.rs` by source inspection only.

## Finding R1 — blocking: `&str` is not the C preprocessor string-literal macro interface

`__DEFINE_ASM_GPR_NUMS` is not a data object in the Linux source.  It is a
function-like-in-use/preprocessor object-like macro whose replacement list is
adjacent C string-literal tokens.  At every use, preprocessing inserts those
tokens directly into the particular inline-assembly template along with
consumer-specific literal fragments and macro interpolation.  The macro has no
Rust value, storage, or call boundary.

The candidate instead exports `pub const __DEFINE_ASM_GPR_NUMS: &str`.  That
has a `&'static str` expression interface, not a token-level string-literal
interface.  Rust does not concatenate adjacent string literals, and a Rust
inline-assembly template must be supplied as a compile-time template literal
(or an equivalent macro expansion), rather than by reading a `&str` constant.
Consequently a future translation of the selected consumers cannot preserve
their direct composition by substituting this constant for the C macro.

The source contexts establish that this is operative, not documentary:

- `asm-extable.h` prefixes `__DEFINE_ASM_GPR_NUMS` to an exception-table
  template that uses C `#gpr` stringification and `__stringify` interpolation;
- `sysreg.h` composes it with `.macro mrs_s`/`.macro msr_s` definitions and
  escaped assembler parameters (`\\sreg`, `\\rt`), then applies `mrs_s` or
  `msr_s` in the same template;
- `fpsimd.h` appends templates containing named inline-assembly operands such
  as `%[pzt]`; and
- `kvm/pauth.c` appends `%[Rd]`, `%[Rn]`, and `%[Rm]` operand interpolation.

The candidate bytes themselves correctly represent the C macro's emitted
directive text: Rust `\\num` produces the required literal assembler `\\num`,
and its tabs and newlines match the adjacent C literals.  A `&str` also has a
static lifetime, so lifetime is not the defect.  It remains unusable at the
required composition point and therefore does not preserve the source
interface or each-use assembler-definition semantics.  Replacing it with a
macro/token mechanism that expands into a compile-time template literal at
each consuming Rust `asm!`/assembly construction site requires a source-level
design consistent with those consumers; this constant cannot be accepted as
that mechanism.

**Disposition:** reject the current candidate; resolve the template-composition
interface before S000071 can be accepted.  No compiler, formatter, linker,
test, or runtime command was used.
