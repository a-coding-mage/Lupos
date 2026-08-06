# Rust review — S018281

Reviewed `src/security/selinux/include/initcalls.rs` independently against
`vendor/linux/security/selinux/include/initcalls.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen x86_64 configuration,
the Phase 0 scope/symbol/ABI/lifetime records, and the direct SELinux users.
No compiler, formatter, linker, test, or Rust-analyzer diagnostic was run.

## Result

No Rust source finding.

* The eight public declarations preserve the upstream names, zero-argument
  `int` signature (`core::ffi::c_int`), and C ABI.  They do not fabricate
  ownership, references, layouts, allocation, or `Send`/`Sync` claims.
* `unsafe extern "C"` correctly keeps calls across this symbol boundary
  explicit.  Call sites translated from `initcalls.c` must retain their
  initialization-phase and configuration preconditions in their own local
  `SAFETY` rationale; this header has no pointers or per-call Rust-side
  invariants to encode.
* The C header declares every symbol unconditionally.  In particular,
  `sel_ib_pkey_init` must remain declared even though
  `CONFIG_SECURITY_INFINIBAND` is absent, while the frozen configuration has
  `CONFIG_NETFILTER=y`; the C callers, rather than this header, select the
  configuration-dependent calls.  Adding Rust `cfg` gates here would diverge.
* Provenance is exact and immutable, and the candidate contains no tests,
  placeholders, panics, broad unsafe blocks, or unauthorized branding.

## Required closure note for the applier

`rewrite/SYMBOLS.tsv` still records this task's include-guard and guard macro
rows as `PENDING_REVIEW`.  Before `DONE`, close those records as the normal
header-guard mapping (no Rust runtime/ABI artifact) and retain this evidence;
the queue protocol forbids leaving a task-local pending semantic record open.

## Evidence

* Upstream header: `vendor/linux/security/selinux/include/initcalls.h:6-19`.
* Direct declaration consumers: `security/selinux/initcalls.c`, `hooks.c`,
  `selinuxfs.c`, `netport.c`, `netnode.c`, `netif.c`, `netlink.c`, and
  `ibpkey.c`.
* Defining Rust-translation tasks are in-scope built-in core files, including
  S018272, S018292, S018293, S018295, S018296, S018297, and S018299; later
  definitions must continue to provide the same C-symbol contract.
