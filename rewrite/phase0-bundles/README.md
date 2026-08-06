# Materialized Phase 0 cache bundles

`BUNDLES.tsv`, `MEMBERS.tsv`, and the deterministic split `*.tar.gz.partNNN`
archives
are the compact Git representation of the raw Phase 0 manifests and Kbuild
metadata. They are not an alternate source of truth: their member hashes must
match the identity-bound raw bytes after materialization.

Restore them locally without invoking an AI, compiler, Kbuild, or Lupos:

```bash
python3 tools/phase0_materialize.py materialize --rewrite rewrite
python3 tools/phase0_materialize.py verify --rewrite rewrite
python3 tools/phase0_materialize.py dematerialize --rewrite rewrite
```

After an authorized fresh Phase 0 extraction has produced raw manifests and
metadata, refresh the committed archives deterministically:

```bash
python3 tools/phase0_materialize.py bundle --rewrite rewrite
python3 tools/phase0_materialize.py verify --rewrite rewrite
```

The raw materialized paths are intentionally ignored by Git. The workflow queue,
identity, frozen configurations, compiler-predicate evidence, and per-task
evidence are separate, directly tracked records.

`dematerialize` verifies each raw member before removing only those cache files.
It leaves every directly tracked Phase 0 and Phase 1 record intact; later
`materialize` restores the exact bundled bytes.
