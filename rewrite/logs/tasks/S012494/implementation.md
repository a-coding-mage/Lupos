# Implementation record

- Task: `S012494`
- Attempt: `1`
- Pipeline: `P02`
- Linux source: `include/acpi/proc_cap_intel.h`
- Destination: `src/include/acpi/proc_cap_intel.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `x86_64`

The pinned header is a guarded macro-only capability-mask definition. The
destination preserves every leaf bit and each composed mask as public `u32`
constants. The `u32` representation follows the pinned x86 consumers, which
operate on `u32 *cap` and `uint32_t *buf`; composed masks retain the source
bitwise-OR expressions and therefore their evaluation and values.

No compiler, formatter, test, runtime, or historical Lupos source was used.
