# Phase 0 / translation boundary

Phase 0 records mechanically selected Linux inputs from the pinned Kconfig and
Kbuild metadata. It does not infer Linux semantics.

The metadata pass preserves source paths, emitted objects, module or built-in
disposition, architecture membership, compile commands, generated sources,
generated headers, `.cmd` files, depfiles, and include dependencies. Drivers
remain original Linux objects and architecture assembly remains mechanically
preserved.

`SYMBOLS.tsv`, `ABI.tsv`, `LIFETIMES.tsv`, and `DRIVER_ABI.tsv` use
`PENDING_REVIEW` when metadata cannot prove a semantic fact. Implementers and
the two independent reviewers must resolve those records from the complete
pinned Linux context before the applier can mark a task `DONE`.
