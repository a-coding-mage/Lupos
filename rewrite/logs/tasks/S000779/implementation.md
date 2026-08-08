# S000779 implementation

- Task: `S000779`
- Pipeline/attempt: `P02` / `1`
- Linux source: `arch/x86/include/uapi/asm/ldt.h`
- Destination: `src/arch/x86/include/uapi/asm/ldt.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `x86_64`

The pinned header defines the LDT entry limits and three modify_ldt contents
values, plus `struct user_desc`.  Its three leading unsigned-int members and
the following unsigned-int bit-field allocation unit are represented by four
`u32` fields under `#[repr(C)]`.  The seven x86_64 bit-fields are exposed by
mask/shift accessors over that allocation unit; the `lm` bit is included for
the selected `__x86_64__` branch.  The source guard and assembler exclusion
are represented by the Rust module boundary, while all selected UAPI values
and names are retained.

No compiler, formatter, linker, test, runtime, or historical Lupos source was
used.  Candidate snapshot: `candidate.diff`.
