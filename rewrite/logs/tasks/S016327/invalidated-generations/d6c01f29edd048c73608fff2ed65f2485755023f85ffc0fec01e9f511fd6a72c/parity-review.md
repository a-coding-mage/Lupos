# Parity review — S016327 (slot 1)

## Verdict

ACCEPT — no parity findings.

## Sources and scope checked

- Pinned source: `vendor/linux/include/uapi/linux/personality.h` at
  `425f94c2954b1fe80ebdbf9b29854e89750355df`, which agrees with
  `vendor/linux.SHA` and the immutable provenance in the candidate.
- Candidate: `src/include/uapi/linux/personality.rs`.
- Queue/SCOPE/FILE_MAP identify this as the `common`, low-risk
  `RUST_TRANSLATE` mapping for the complete UAPI header.  Header-closure
  evidence selects it for both frozen configurations (aarch64: 8,744
  consumers; x86_64: 2,852 consumers).
- The symbol inventory records precisely the two anonymous enums and the
  `PER_CLEAR_ON_SETID` macro for both architectures.  There are no selected
  configuration branches in the source header.
- Consumer context was checked in `include/linux/personality.h`,
  `arch/x86/kernel/process.c`, `arch/x86/kernel/process_64.c`,
  `arch/x86/mm/mmap.c`, `arch/x86/include/asm/elf.h`,
  `arch/arm64/kernel/process.c`, `arch/arm64/include/asm/elf.h`,
  `fs/exec.c`, `fs/binfmt_elf.c`, `kernel/sys.c`, and
  `security/commoncap.c`.

## Exhaustive comparison

The candidate exports every source public constant with its spelling intact:

- Flag enum: `UNAME26`, `ADDR_NO_RANDOMIZE`, `FDPIC_FUNCPTRS`,
  `MMAP_PAGE_ZERO`, `ADDR_COMPAT_LAYOUT`, `READ_IMPLIES_EXEC`,
  `ADDR_LIMIT_32BIT`, `SHORT_INODE`, `WHOLE_SECONDS`, `STICKY_TIMEOUTS`, and
  `ADDR_LIMIT_3GB` retain their exact hexadecimal values.
- `PER_CLEAR_ON_SETID` retains the original four operands and OR order:
  `READ_IMPLIES_EXEC | ADDR_NO_RANDOMIZE | ADDR_COMPAT_LAYOUT |
  MMAP_PAGE_ZERO` (value `0x0740000`).
- Personality enum: `PER_LINUX`, `PER_LINUX_32BIT`, `PER_LINUX_FDPIC`,
  `PER_SVR4`, `PER_SVR3`, `PER_SCOSVR3`, `PER_OSR5`, `PER_WYSEV386`,
  `PER_ISCR4`, `PER_BSD`, `PER_SUNOS`, `PER_XENIX`, `PER_LINUX32`,
  `PER_LINUX32_3GB`, `PER_IRIX32`, `PER_IRIXN32`, `PER_IRIX64`,
  `PER_RISCOS`, `PER_SOLARIS`, `PER_UW7`, `PER_OSF4`, `PER_HPUX`, and
  `PER_MASK` all retain their source literals and source OR expressions.

Every source enumerator is representable as a C `int` on both selected
architectures; explicit `i32` constants therefore preserve both the C enum
expression width and all bitwise-OR results.  The header has no complement,
shift, cast, signed-overflow, alias, or architecture/configuration conditional
expression to translate.  The candidate introduces no replacement behavior,
storage, functions, unsafe code, or tests.

The checked consumers use these values as integer masks/flags (including
`~READ_IMPLIES_EXEC` in architecture personality updates); the explicit `i32`
representation supplies the correct signed operand category for that usage.
The source has only the syscall-note SPDX line and no additional copyright
notice; the candidate preserves that SPDX identifier, exact Linux source path,
revision, `common` architecture scope, and stable task ID.  No branding delta
or public-name omission was found.

## Manifest semantic records

The ABI/LIFETIMES entries marked `PENDING_REVIEW` concern the two anonymous
constant-only C enums.  This review establishes that neither declares storage,
layout, ownership, linkage, or lifetime: they are solely sets of `int`
enumerator constant expressions.  The applier can close those records on this
evidence without an ABI/lifetime design decision.

No source, manifest, or candidate changes were made by this reviewer.  No
build, formatter, compiler, test, emulator, debugger, or runtime command was
run.
