# Implementation evidence

- Task: `S000730`, attempt 1, pipeline `P02`.
- Source: `vendor/linux/arch/x86/include/asm/trapnr.h` at Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`.
- Destination: `src/arch/x86/include/asm/trapnr.rs`.
- Architecture: `x86_64`, matching the frozen task row.
- The complete pinned header was read. It contains one include guard, eight
  FRED/virtualization event type macros, and twenty-four trap-number macros.
- The destination preserves every public macro spelling and numeric value as
  an explicitly typed `i32` constant. The source macros are integer literals
  with no expression operands, conditional branches, or side effects.
- No historical Lupos source, compiler, formatter, test, or runtime command
  was used.
