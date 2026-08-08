Task S018288 implementation evidence

Source: vendor/linux/security/selinux/include/policycap.h
Revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
Destination: src/security/selinux/include/policycap.rs
Architecture: x86_64

The source is a declaration-only SELinux policy-capability header. The fresh
Rust file preserves all fifteen anonymous C enum constants, the sentinel, the
sentinel-minus-one macro, and the external fifteen-element name array. C enum
constants are represented as i32 constants so their integer namespace and
zero-based values remain explicit; the array length uses the required usize
type-level cast without changing the exported array ABI. No conditional branch
is present in the pinned source.

Direct pinned context checked: policycap_names.h defines the matching external
array; security.h indexes selinux_state.policycap with these constants;
selinuxfs.c iterates through POLICYDB_CAP_MAX and the array; selinux/Makefile
builds the SELinux object under CONFIG_SECURITY_SELINUX.
