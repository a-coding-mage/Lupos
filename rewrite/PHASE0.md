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

## Compiler predicate inventory

Compiler builtins used by mechanically selected source or its selected headers
are Phase 0 inputs, not semantic notes. `tools/compiler_predicates.py --execute`
discovers `__has_attribute`, `__has_builtin`, `__has_feature`,
`__has_extension`, `__has_c_attribute`, `__has_declspec_attribute`, and
`__has_warning` expressions from the selected source/header closure. It takes
the authoritative per-architecture Kbuild command, replaces its source/output
operation with a generated direct predicate probe, and requests preprocessing
only. It never compiles an object or executes generated code.

The canonical evidence is `rewrite/compiler-predicates/`: its TSV, fingerprint,
command records, probes, raw stdout/stderr, and `VALIDATION.tsv`. Every row
records compiler identity, target, configuration and toolchain hashes, the
original command identity, probe/result hashes, exit status, timestamps, source
locations, and architecture. `tools/validate_compiler_predicates.py --execute`
reconstructs the probe and Kbuild context independently and replays each
proven row. Compiler documentation, parsing an attributed declaration, and a
generic host-only `clang -E` invocation are insufficient.

`PHASE0_IDENTITY.tsv` binds the inventory fingerprint, schema, row counts, and
independent-validation status. A changed predicate set/result, compiler or
compiler hash, relevant flags, target, or configuration invalidates Phase 0 and
requires a fresh manifest and queue. A predicate affecting selected code may
not remain `PENDING_REVIEW`.
