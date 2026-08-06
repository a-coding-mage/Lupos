# Phase 0 / translation boundary

Phase 0 records mechanically selected Linux inputs from the pinned Kconfig and
Kbuild metadata. It does not infer Linux semantics.

The metadata pass preserves source paths, emitted objects, module or built-in
disposition, architecture membership, compile commands, generated sources,
generated headers, `.cmd` files, depfiles, and include dependencies. Drivers
remain original Linux objects and architecture assembly remains mechanically
preserved.

Selected C enumerators are mechanical symbol facts. `SYMBOLS.tsv` records each
enumerator and every value that can be proven from an implicit sequence or a
restricted integer expression over preceding enumerators; unsupported values
remain `PENDING_REVIEW`. Header-provider ordering treats enumerator identifiers
as definitions alongside operative macros, types, and functions. This is
required when a Linux header consumes an enum constant established earlier by
the compiler dependency context without directly including its provider.

`SYMBOLS.tsv`, `ABI.tsv`, `LIFETIMES.tsv`, and `DRIVER_ABI.tsv` use
`PENDING_REVIEW` when metadata cannot prove a semantic fact. Implementers and
the two independent reviewers must resolve those records from the complete
pinned Linux context before the applier can mark a task `DONE`.
