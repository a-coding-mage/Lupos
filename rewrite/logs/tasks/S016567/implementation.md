# S016567 implementation — attempt 2

- Task: `S016567`
- Pipeline: `P02`
- Role: `implementer`
- Model: `gpt-5.6-terra`
- Effort: `medium`
- Linux source: `vendor/linux/include/xen/interface/features.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/xen/interface/features.rs`
- Architectures: `aarch64`
- Phase 0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- Scope fingerprint: `b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0`
- Symbols fingerprint: `7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`
- ABI fingerprint: `ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`
- Lifetimes fingerprint: `0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`

The complete pinned header's active feature-index macros are public `i32`
constants, retaining the C `int` numeric values and bit-index semantics. The
commented-out deprecated `XENFEAT_grant_map_identity` is not an active C macro
and has no Rust definition. Include guards are represented by Rust modules.

This declaration-only header contains no ABI layout, ownership, allocation,
locking, RCU, refcount, or error-path behavior. No compiler, formatter, linker,
test, runtime, or diagnostic tool was run.
