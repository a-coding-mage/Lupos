# S016178 implementation

- Task: `S016178` / `include/uapi/linux/if_vlan.h` → `src/include/uapi/linux/if_vlan.rs`
- Pinned Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Queue architecture class: `common` (selected by both frozen `x86_64` and
  `aarch64` configurations)
- Queue lease: `P01`, owner `p01-terra-fallback`, attempt `1`

The fresh UAPI translation preserves all three C enum tag surfaces as C `int`
aliases and every enumerator as an unqualified constant with its original
implicit or explicit value. It preserves the named `vlan_ioctl_args` C layout:
the `int` command, both 24-element C-character arrays, anonymous C union
(represented by the required Rust union name and retained `u` field), all union
members with their original signedness, and the trailing C `short`.

`#[repr(C)]` is applied to both layout-bearing declarations. No configuration
conditional selects a different declaration in either frozen architecture.
No build, compiler, formatter, test, or runtime command was run.
