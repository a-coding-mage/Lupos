# Rust source review — S016353 — attempt 1 — slot 2

Reviewer role: `rust_reviewer`  
Model/effort: `gpt-5.6-terra` / `high`  
Scope reviewed: `vendor/linux/include/uapi/linux/reboot.h` and
`src/include/uapi/linux/reboot.rs`, plus pinned in-tree consumers establishing
the syscall parameter types. No compiler, formatter, test, analyzer, or
historical Lupos source was used.

## Finding R1 — C integer-expression types are erased at the public UAPI boundary

The candidate declares every macro as `u32` and states that all retain unsigned
32-bit values. That is not the type contract of the source macros. On the
approved ABIs, the unsuffixed decimal literals and hexadecimal literals that fit
in `int` are signed `int`; the hexadecimal literals above `INT_MAX` select
`unsigned int`. In particular, source lines 10--13 define signed `int` magic
values, while line 9 selects `unsigned int`; command literals on lines 29, 32,
33, and 36 are signed `int`, whereas lines 30, 31, 34, and 35 select `unsigned
int`.

This distinction is observable at the pinned syscall boundary:
`vendor/linux/kernel/reboot.c:728` declares `magic1` and `magic2` as `int`, but
`cmd` as `unsigned int`; its comparisons at lines 740--744 use the C usual
arithmetic conversions. The pinned nolibc wrapper also takes all three values
as `int` (`vendor/linux/tools/include/nolibc/sys/reboot.h:24`) and passes the
two magic macros directly at line 31. Exposing every macro as `u32` removes the
signed literals and forces Rust consumers either to fail to type-match signed
parameters or to introduce a conversion whose overflow/sign semantics are not
encoded by this UAPI translation. It therefore does not preserve the source
macro expression and syscall ABI contract even though each low 32-bit bit
pattern matches.

Required resolution: retain each source macro's signedness/type semantics (or
provide source-proven, explicitly typed UAPI bindings that preserve the exact
conversion at each `int`/`unsigned int` syscall parameter). Do not use one
uniform `u32` type merely because every value occupies 32 bits.

Affected semantic records:

- `SC1-8a6652527fa40ccada1e73eb5eb45b033137662045abbaa5716554370ab81c73`
- `SC1-4a7e064e0852b0600fae35adf08ea1b5f981c1e43c4efadcfd0f933c2735ece5`
- `SC1-1ef527822335678afcfe3823bec42b82cda6b1f70fff75c0173c2e0a535683e2`
- `SC1-a51cc128653a945818237e9f52f25ae225f11fd23bd8eb51b65faf6c71c6031c`
- `SC1-f02ace93828925b3b1d98e7f71f9cd7edbdd93d36ac5c81f9381ca8ba73768a8`
- `SC1-234e41f48b37edd6a73604956692298b955347d24415f32fda198fd1f2448958`
- `SC1-00b3c7f72a0e97922de0a6a92917f8187724123d9166136d4e7183fc270ecbd9`
- `SC1-618927f3dd15d10d058cd661de0afed0feec3730564b509410acc1b6c2c2392e`
- `SC1-72fde2a41c0c31e75eac2daa54534d1bc411b370e0a9cbda5c7f6da4dbef741c`
- `SC1-19c6b1b1d31c979c28f0d9f9f9d6fc5c10dd2628d556e9d62bdcb56f899dfc72`
- `SC1-8946c9c42e33fdd0ecbb19fb8370fd35f52ca14f99b6e856c9c20ca9d09f5dcc`
- `SC1-6a487b252895e922a80fe7f081fb9700553d8c29411f9b56e6ed86c842d53f7a`
- `SC1-78944cba85b4b3c065ee5898189d9b3fe739d16a5a2f91e03d084fb04d879aa8`
- `SC1-c93f2b3d3fa697dc602edead75c883c86d099cd44f657e9b35182740f09ae660`
- `SC1-0b7999ddac0ad1fa65198ec4f1cb554aba21474b3ba10f4d035ed7d2a293cd64`
- `SC1-24f71a375162a380f412cfdf423216b2aedb1930b75eda5be177889f90737973`
- `SC1-1a22ed227ef37b7ebfb01248a2ec9855ebdc2927c1dfc0751e0b15533eec2b81`
- `SC1-0af6be4f0a6434b0a2e4890e7cd4a1e65ef641436ae66346c49561f1ad0b6a5b`
- `SC1-767a1e935a441c35716d6b2f235f3fb3e21670fbeb225dcc2acfc5728bef351c`
- `SC1-9deb95c6d86102797951860c28f68aa38f5097107feafd4f11e1adfb6ceb47f4`
- `SC1-177a5e7911a4aa8a39b78eae8d35634dc15c738272809ac513df22b6d19c5597`
- `SC1-62acd7d7286be2e9d0d2fd93b499c0529126823e001f8561526627792b71ee62`
- `SC1-5bd6a384fcb0d5049fcf58a15a3e4ed5cdfcfc66ccafcbfdbdbc5a04c9ef44be`
- `SC1-13843d938cd2460b4a9a357ba8d9ffaf9a22a478d4d706c155991b3ac0b978d8`
- `SC1-488e2a712f5bc9488a521fb76d2ebca81a6adf2fdcb56764eb2c4afe16fdf93f`
- `SC1-f36afad2755e41e7f4895d9673f63279f9e72153c44562ec6cb36887ff34f854`

## Other Rust-mechanism checks

The source and candidate contain no data layout, FFI declaration, pointer,
reference, ownership, aliasing, pinning, interior-mutability, `Send`/`Sync`,
callback, RCU/refcount, allocation, `Drop`, or `unsafe` mechanism to audit.
There are no enum declarations or function-like macros to map. The candidate
does retain all twelve value names and their low 32-bit values. The C include
guard is a textual-preprocessor mechanism; its absence is not independently
reported here because Rust module loading is not textual re-inclusion. The
type-erasure finding above remains unresolved and prevents approval.

Review result: **FINDINGS** (`R1`).
