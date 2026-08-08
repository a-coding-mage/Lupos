# S016464 implementation

- task: S016464
- attempt: 1
- pipeline: P02
- role: implementer
- model: gpt-5.6-luna
- reasoning_effort: medium
- linux_path: include/uapi/linux/virtio_ids.h
- destination_path: src/include/uapi/linux/virtio_ids.rs
- linux_revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
- architectures: common
- decision: COMPLETE

The complete pinned header was read. It contains only an include guard,
license text, and 46 object-like integer macros: 39 current Virtio device IDs
and 7 transitional IDs. Each unsuffixed C integer constant is represented as a
public Rust `i32` constant with the same identifier and value. The include
guard has no runtime Rust representation. No configuration branch changes the
selected definitions.

No unsafe code, calls, allocation, locking, cleanup, or architecture-specific
behavior is present in this header. The BSD license notice is retained in the
destination provenance and source comments.

Frozen identity bindings:

- phase0 identity: 0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2
- queue: cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f
- scope: b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a
- symbols: 7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf
- ABI: ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39
- lifetimes: 0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8

The candidate is source-only; no compiler, formatter, linker, runtime, test,
or benchmark command was run.
