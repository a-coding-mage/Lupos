# S018289 implementation

Oracle: `vendor/linux/security/selinux/include/policycap_names.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The selected x86_64 header defines one externally linked `const char *const`
array, `selinux_policycap_names`, containing the names for all 15 values before
`__POLICYDB_CAP_MAX` in the dependent `policycap.h` task (S018288).  The Rust
definition retains the C symbol name, 15-element pointer-array representation,
entry order, and NUL-terminated ASCII contents.  Its transparent element
wrapper has the same representation as a C character pointer; its `Sync`
justification is limited to the immutable static string addresses held by this
const array.

The header guard and clang-format directives have no Rust runtime or ABI
counterpart.  No configuration conditional surrounds this definition in the
selected x86_64 preprocessing context.  Pinned source callers include the
array directly through `services.c`; I also inspected the SELinux capability
consumers in `ima.c` and `selinuxfs.c` to confirm that element ordering and
the terminating NUL representation are the operative contract.
