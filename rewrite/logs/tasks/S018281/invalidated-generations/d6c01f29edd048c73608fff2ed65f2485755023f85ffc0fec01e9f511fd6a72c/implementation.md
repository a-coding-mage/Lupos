# Implementation: S018281

- Task: `security/selinux/include/initcalls.h` → `src/security/selinux/include/initcalls.rs`
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `x86_64`
- Implementer model/effort: `gpt-5.6-terra` / `medium` (fallback because Luna was unavailable)

Read the complete pinned header, each SELinux implementation that defines one
of its declarations, its seven selected header-closure consumers, and the
frozen x86_64 configuration.  The header has only its C include guard and
eight unconditional external declarations, each with C `int` return type and
an empty `void` parameter list.  The Rust mapping is one `unsafe extern "C"`
declaration per original identifier, each returning `c_int`; no declaration
has parameters, storage, a layout, or an init-section attribute in this header.

The `CONFIG_NETFILTER=y` initcall consumer conditionally calls
`selinux_nf_ip_init`; `CONFIG_SECURITY_INFINIBAND` is absent, so the selected
initcall consumer does not call `sel_ib_pkey_init`.  Both declarations remain
unconditional because the original header declares both unconditionally.  The
C include guard prevents repeated textual inclusion only and has no Rust
runtime or ABI counterpart.

No build, formatter, compiler, test, or runtime command was run.
