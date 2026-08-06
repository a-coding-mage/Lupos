# Parity review — S018281

Reviewed `src/security/selinux/include/initcalls.rs` against pinned
`security/selinux/include/initcalls.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Result: **no parity findings**.

The candidate preserves the upstream SPDX identifier and the required immutable
provenance for task S018281 and x86_64.  It declares every eight upstream
externally linked, zero-argument `int` function with the same symbol spelling:
`init_sel_fs`, `sel_netport_init`, `sel_netnode_init`, `sel_netif_init`,
`sel_netlink_init`, `sel_ib_pkey_init`, `selinux_nf_ip_init`, and
`selinux_initcall`.  `core::ffi::c_int` is the x86_64 C `int` ABI type, and the
`unsafe extern "C"` block retains C linkage without introducing a renamed
symbol.

The C include guard carries no additional run-time or ABI state; the Rust module
is defined once.  The source header makes all eight declarations unconditional,
which the candidate retains.  This is correct for the frozen x86_64 context:
`CONFIG_SECURITY_SELINUX=y` selects the header, while `initcalls.c` alone gates
the InfiniBand and NETFILTER calls.  `CONFIG_NETFILTER=y`; no
`CONFIG_SECURITY_INFINIBAND` assignment is present.  Direct selected SELinux
consumers and the `initcalls.c` call sites are consistent with these
declarations.

No compiler, formatter, linker, test, or runtime tool was invoked.
