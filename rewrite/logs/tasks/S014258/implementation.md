# Implementation: S014258

Source: `vendor/linux/include/linux/lsm/apparmor.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

The complete source defines one forward declaration and one conditional member
in `struct lsm_prop_apparmor`.  Neither frozen configuration defines
`CONFIG_SECURITY_APPARMOR`; consequently, the selected definition has no
members on x86_64 or AArch64.  The Rust destination uses `#[repr(C)]` and an
empty struct to preserve that configuration-selected representation.  The C
forward declaration has no selected use when the member is absent.

No executable logic, exported function, allocation, synchronization, or
cleanup path is present in this header.
