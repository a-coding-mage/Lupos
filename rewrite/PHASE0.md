# Phase 0 identity and metadata procedure

Phase 0 freezes one tuple: the pinned `vendor/linux` commit, one explicit
toolchain policy, the x86_64 and AArch64 configurations produced under that
policy, and the material Kbuild environment. `rewrite/toolchain/` records exact
paths, versions, and environment values. `rewrite/PHASE0_IDENTITY.tsv` binds
those records and both configuration hashes to the extractor and queue schema.

Configuration synchronization is permitted only in Phase 0. Each architecture
must record the before/after configuration hashes and every changed symbol in
`config-transition.tsv`. The first synchronized result may be adopted as the
frozen configuration only after it is documented and a second synchronization
produces byte-for-byte and semantic equality. A toolchain or configuration
change invalidates the identity, scope, manifests, queue, and fingerprint.

The metadata-only Linux pass may produce generated headers/sources, Kbuild
`.cmd` files, depfiles, object/module membership, compile commands, and related
selection evidence. It must not compile or execute Lupos. Provisional invalid
runs are archived under `rewrite/archive/` and invalidated through the queue
tool before authoritative outputs are regenerated.

The canonical toolchain is the complete LLVM 19 suite under
`/usr/lib/llvm-19/bin/`. Every invocation uses the absolute
`LLVM=/usr/lib/llvm-19/bin/` value and `LLVM_IAS=1`; Rust-distributed linkers
are rejected even when visible on `PATH`.
