# S012620 implementation — attempt 1

- Task: `S012620`
- Pipeline: `P01`
- Role: `implementer`
- Model: `gpt-5.6-terra`
- Effort: `medium`
- Linux source: `vendor/linux/include/crypto/dh.h`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Destination: `src/include/crypto/dh.rs`
- Architectures: `aarch64`
- Phase 0 identity: `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`
- Queue fingerprint: `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`
- Scope fingerprint: `b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a`
- Symbols fingerprint: `7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`
- ABI fingerprint: `ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`
- Lifetimes fingerprint: `0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`

Translated the complete selected header: `struct dh` remains a C-layout
three-pointer/three-`unsigned int` record, and all three C declarations retain
their exact exported C names, pointer mutability, `unsigned int` widths, and
`int` returns. The decoder's C aliasing contract is retained as raw pointers:
it makes `params` point into the caller-owned packet buffer rather than taking
or allocating its storage. Include guards are represented by Rust module
inclusion and emit no item. No compiler, formatter, linker, test, runtime, or
diagnostic tool was run.
