# S016368 implementation

task: S016368
pipeline: P01
lease: verified for `pipeline_coordinator` through 2026-08-08T09:04:19.378Z
source: `vendor/linux/include/uapi/linux/securebits.h`
destination: `src/include/uapi/linux/securebits.rs`
architectures: common (`x86_64`, `aarch64`)
linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`

## Frozen evidence binding

The implementation was performed against the frozen Phase 0 identity and
manifests:

- identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- queue: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- scope: `b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a`
- symbols: `7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`
- ABI: `ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`
- lifetimes: `0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`

## Source review

The complete pinned header was read. It contains one include guard, the
`issecure_mask` expression, the secure-bit index constants, the corresponding
bit-mask expressions, and the aggregate masks. All selected symbols and both
architecture branches are represented. The C unsuffixed integer expressions
are represented as `i32`; `issecure_mask` preserves the C `1 << X` result and
is a const function so aggregate masks remain compile-time expressions.

Closure: `COMPLETE/SOURCE_REVIEWED_VALUE`.
