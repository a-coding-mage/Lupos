# Parity review — S016320

Reviewed `vendor/linux/include/uapi/linux/oom.h` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/oom.rs` for the frozen `common` x86_64/aarch64 union.

## Result

No parity findings.

## Source comparison

- The source SPDX identifier is retained exactly:
  `GPL-2.0 WITH Linux-syscall-note`.  The candidate provenance identifies the
  exact Linux source, revision, `common` architecture membership, and task.
- The source's only operative UAPI definitions are represented under the same
  public names: `OOM_SCORE_ADJ_MIN`, `OOM_SCORE_ADJ_MAX`, `OOM_DISABLE`,
  `OOM_ADJUST_MIN`, and `OOM_ADJUST_MAX`.  No source macro is omitted and the
  candidate adds no operative UAPI constant.
- Linux defines the expressions `(-1000)`, `1000`, `(-17)`, `(-16)`, and `15`.
  Each has C `int` type on both selected architectures; the candidate exposes
  the exact signed values as `core::ffi::c_int`, the Rust representation of the
  selected targets' C `int` ABI.
- The C include guard has no runtime or exported-UAPI value after a header is
  included.  Rust module loading supplies the corresponding one-definition
  namespace behavior; no C-linkage symbol is required because these are C
  preprocessor macros, not objects or functions.
- The direct kernel wrapper `include/linux/oom.h` includes this UAPI header.
  Active uses in `fs/proc/base.c` preserve the legacy range and scaling
  arithmetic, while `mm/oom_kill.c` compares the minimum score adjustment.
  All five translated values and their signed `int` semantics match those
  contexts.  `CONFIG_PROC_FS=y`, `CONFIG_MMU=y`, and (on aarch64)
  `CONFIG_MEMCG=y` in the frozen configurations do not conditionalize any
  definition in this UAPI header.

Manual source review only; no compiler, formatter, analyzer, build, or test
command was used.
