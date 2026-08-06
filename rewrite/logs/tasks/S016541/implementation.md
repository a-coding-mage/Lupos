# S016541 implementation

Translated `include/vdso/time64.h` from pinned Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` to
`src/include/vdso/time64.rs`.

The header contains eight selected object-like conversion macros and no
functions, storage, layouts, or exported linkage.  Each is represented as a
public Rust constant with its original C literal category retained:
`long` maps to `core::ffi::c_long`, and `long long` maps to
`core::ffi::c_longlong`.  Both approved targets are 64-bit (x86_64 and
aarch64), so all eight source values are exactly representable.

Source inspection covered the complete header, both frozen configuration
memberships in the scope/symbol records, the header-closure consumers, and the
vDSO users `include/vdso/jiffies.h` and `lib/vdso/gettimeofday.c`.  This task
has no ownership, lifetime, synchronization, FFI layout, or callable ABI
contract beyond preserving the constant values and integer categories.

No compiler, formatter, test, analyzer, or runtime command was run.
