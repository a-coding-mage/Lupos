# Implementation — S014261

- Linux source: `vendor/linux/include/linux/lsm/smack.h`
- Destination: `src/include/linux/lsm/smack.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architectures: `x86_64,aarch64` (common header)

The complete upstream header declares `struct smack_known` and defines
`struct lsm_prop_smack`. Its only member, `struct smack_known *skp`, is inside
`#ifdef CONFIG_SECURITY_SMACK`. Both frozen configurations record
`CONFIG_SECURITY_SMACK` as not set, so the selected definition is a C
`repr(C)` empty struct. No Smack pointer declaration or storage is selected.

No compiler, formatter, build, test, debugger, or historical Lupos Rust
source was used.
