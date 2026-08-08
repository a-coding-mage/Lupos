# S016178 implementation

- Task: `S016178`
- Pipeline/attempt: `P02` / `1`
- Linux source: `vendor/linux/include/uapi/linux/if_vlan.h`
- Destination: `src/include/uapi/linux/if_vlan.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `common` (frozen x86_64/AArch64 union)
- Implementer: Luna, medium effort

The fresh translation preserves all three C enums and their implicit integer
values, the five VLAN flag values, the UAPI `int`/`unsigned int`/`short` field
widths, the 24-byte character arrays, and the anonymous union's overlapping
storage as the explicitly named `vlan_ioctl_args_u` Rust union. `#[repr(i32)]`
keeps each C enum's int representation and `#[repr(C)]` preserves structure and
union layout. No selected source branches beyond the include guard exist.

No compiler, formatter, test, runtime, or historical Lupos source was used.
