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
selection evidence. It must not compile or execute Lupos. Raw Phase 0 manifests
and Kbuild metadata are local materialized caches: their deterministic,
checksummed gzip bundles are committed under `rewrite/phase0-bundles/`, while the
uncompressed files are ignored. This avoids duplicating multi-gigabyte generated
data in Git without weakening reproducibility or Phase 0 identity binding.

Use the following deterministic, no-AI commands after obtaining the repository
or after a fresh Phase 0 extraction:

```bash
python3 tools/phase0_materialize.py materialize --rewrite rewrite
python3 tools/phase0_materialize.py verify --rewrite rewrite
python3 tools/phase0_materialize.py dematerialize --rewrite rewrite
python3 tools/phase0_materialize.py bundle --rewrite rewrite
```

`materialize` restores only the bundled raw Phase 0 paths and rejects a
conflicting local file. `verify` checks both compressed bundle hashes and every
raw member hash. `dematerialize` removes only already-verified local cache
members, never identity, queue, predicate, task, `vendor/linux`, or `src`
paths. `bundle` replaces the bundles atomically from an already
materialized Phase 0 result; it neither reads nor modifies `vendor/linux` or
`src/`. It is an artifact-storage operation, not a Phase 0 execution. A fresh
extraction remains the authoritative way to derive new metadata from the frozen
Linux/Kbuild inputs.

Provisional invalidation appends one compact row to
`rewrite/archive/PRUNED_TSVS.tsv`. Per-run copies of manifests, metadata, logs,
or README files are not retained.

The full Kbuild trees under `rewrite/kbuild/` are transient extraction inputs.
They are ignored, never bundled, and never archived. Once the retained metadata
has passed validation, remove them to avoid preserving generated object/image
output; a fresh authorized Phase 0 run regenerates them when needed.

## Immutable semantic base and effective closure ledger

`SCOPE.tsv`, `SYMBOLS.tsv`, `ABI.tsv`, and `LIFETIMES.tsv` remain immutable
mechanical Phase 0 inputs.  Their mechanically unprovable fields begin as
`PENDING_REVIEW`; Phase 1 MUST NOT substitute values directly into those TSVs.
`rewrite/semantic-closure/SCHEMA.tsv` freezes the field-level key formula and
transaction schemas.  `BASE.tsv` binds each base manifest hash, row count,
pending-field count, the complete deterministic task-key-set digest, and the
schema hash.  Both files are authoritative Phase 0 manifests and are bound by
`PHASE0_IDENTITY.tsv`.  `LEDGER.jsonl` is append-only mutable Phase 1 state; its
schema/tool are identity-bound, but its contents are deliberately excluded so
a valid task commit does not invalidate the queue.

For each task, a stable closure key is derived from the key-schema version,
manifest name and hash, one-based TSV data-row position, field name, and task
ID.  Every task-owned base field exactly equal to `PENDING_REVIEW` must appear
once in the implementation proposal.  The exact canonical evidence names are:

```text
semantic-closure-proposal.tsv
semantic-closure-proposal.sha256
semantic-closure-parity-review.tsv
semantic-closure-rust-review.tsv
semantic-closure-final.tsv
semantic-closure-dispositions.tsv
semantic-closure-commit.json
```

The implementer creates and seals the complete proposal against the current
task, attempt, pipeline, queue fingerprint, Phase 0 identity, four base hashes,
Linux SHA, candidate diff, and implementation evidence.  Each independent
review command reads only that proposal and its own fixed report, records any
finding IDs and affected closure keys, and binds its attestation to the same
proposal hash.  The seal is created only after the complete ordered proposal
passes validation, hashes the proposal TSV bytes rather than the seal output,
and is rejected after any proposal-byte change.  A partial or reordered
proposal cannot create an acceptance seal.  The applier receives both
attestations, produces the final
same-key/same-order record set and one structured disposition per finding, and
may change proposed semantic values only for keys authorized by a
`RESOLVED_CHANGED` finding.

`tools/semantic_closure.py commit` takes the existing queue lock, revalidates
all base and evidence hashes, appends a complete `PREPARE` record to the
ledger, fsyncs it, appends `semantic_closure_committed` to `events.jsonl`, then
appends the matching ledger `COMMIT` and writes the task receipt.  Effective
values exist only after the matching commit and event.  A retry finishes an
identical prepared transaction; mismatched attempts, hashes, key sets,
cross-task records, reordered records, unresolved findings, and any remaining
effective `PENDING_REVIEW` are rejected.  `rewrite_queue.py done` requires this
current-attempt committed closure in addition to the ordinary five reports.

`rewrite_queue.py freeze` opens exactly one clean ledger generation for the new
queue fingerprint.  A Phase-gate reopen quarantines all seven semantic evidence
files together with the ordinary task evidence, preserves historical ledger
records as append-only data, and grants no current-generation acceptance
credit.  The validator requires one clean generation, zero current task
commits, all rows `TODO` at attempt zero, and no canonical task-root evidence
before Phase 1 may reopen.

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

The repeatable capture and independent replay invocations are:

```bash
LLVM=/usr/lib/llvm-19/bin/ LLVM_IAS=1 \
  python3 tools/compiler_predicates.py --execute
LLVM=/usr/lib/llvm-19/bin/ LLVM_IAS=1 \
  python3 tools/validate_compiler_predicates.py --execute
```

The capture tool refuses to replace existing evidence. Archive an invalidated
run first, then regenerate the compiler predicates before regenerating the
Phase 0 manifests and queue.

## Header provider graph

Header task ordering is derived from each architecture's retained Kbuild
dependency assignment and exact include search path. Literal include edges are
resolved for both pinned Linux headers and generated headers. Generated and
other non-translated headers remain vertices while their paths are projected to
the first translated header task, so wrappers such as generated `asm/types.h`
cannot erase the `asm-generic/types.h` prerequisite.

Some Linux headers intentionally rely on definitions established earlier by a
consumer rather than including every provider themselves. For every retained
Rust translation-unit context, Phase 0 therefore intersects the header's
lexical type, tag, function, and operative-macro references with the selected
definition inventory. It records the nearest preceding defining header for
references not supplied by the header's architecture-specific direct include
closure, and collapses nested defining headers to the outer sufficient
provider. These architecture-specific relationships and the exact identifiers
they provide live in `metadata/header_context_edges.tsv`. Providers reached
from an explicit frozen `-include` root are distinguished from ordinary
dependency-order providers.

The union of projected literal includes and ordered context providers is
condensed into deterministic strongly connected components. Components are
then linearized into an acyclic task DAG. The independent validator reconstructs
literal resolution, generated-wrapper projection, ordered dependency replay,
lexical definition/reference matching, forced-include ancestry, components,
task reachability, and acyclicity from retained Kbuild evidence; it does not
accept the extractor's graph on trust.

Named C enumerators are provider definitions and must be inventoried with a
mechanically evaluated value whenever their implicit sequence or restricted
integer expression over earlier enumerators permits it. The independent
validator reconstructs both the enumerator inventory and those values directly
from pinned source before accepting any enum-dependent context edge.

A phase-gate defect discovered after source review is invalidated only with
`rewrite_queue.py invalidate --phase-gate-reopen`. That explicit mode still
requires a valid branch and fingerprint, rejects every active stage or lease,
records prior terminal rows in the append-only event log, and never rewrites
the invalid queue TSV. Ordinary provisional invalidation remains stricter and
continues to reject `DONE` rows.

After regenerated manifests pass staged pre-queue validation, the matching
recorded invalidation is consumed by `rewrite_queue.py init
--phase-gate-reopen --archive <recorded-path> --reopen-reason <reason>`. The
tool verifies the superseded immutable fingerprint without trusting a replaced
identity, writes exactly one prune-ledger row, and retains no per-run archive
directory. Before replacing the queue, it moves every canonical root acceptance
file into
`rewrite/logs/tasks/<id>/invalidated-generations/<superseded-fingerprint>/`,
records deterministic file hashes and the superseded queue state in
`QUARANTINE.tsv`, and appends one task-specific quarantine event while holding
the queue lock. Existing `attempts/` evidence and destination source files are
preserved, but neither receives carryover acceptance credit. Every regenerated
task begins at `TODO`/attempt zero, and a fresh claim fails closed if a canonical
acceptance filename unexpectedly remains at the task root.

## Oracle-only test classification

Selected original test material remains inventoried but never becomes a Rust
mapping or queue task. `metadata/oracle_classification.tsv` records the exact
set and the mechanical reason for every `ORACLE_ONLY` row.

Explicit `include/kunit/`, `lib/kunit/`, `tools/testing/`, KUnit-named paths,
and directory components named `test`, `tests`, `testing`, or `selftests` are
oracle structure. Boundary-delimited `test`, `selftest`, and `selftests`
basename tokens identify in-tree test sources outside driver-owned Kbuild
targets. Driver-owned diagnostics with such generic names remain original
Linux driver objects; unrelated production names such as `testmgr`, `memtest`,
`testmode`, and `cabletest` are not test tokens. A selected header used only by
oracle compilation units is retained as oracle support, while a header shared
with production Rust consumers remains production unless its own path is
explicit oracle structure.

Independent validation reconstructs this set from pinned paths, Kbuild owners,
and compiler dependency consumers. It rejects an oracle path in
`RUST_TRANSLATE`, a non-empty `src/` destination, semantic manifests, or the
translation queue, while requiring the selected path to remain represented in
`FILE_MAP.tsv` as inventory evidence.
