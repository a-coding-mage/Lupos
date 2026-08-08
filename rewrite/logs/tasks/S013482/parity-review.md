# Parity review — S013482 / P02 / attempt 1 / slot 1

Reviewed only the current candidate, its candidate diff, the sealed task-local
semantic-closure proposal, frozen manifests, and pinned local Linux source/context.
No compiler, formatter, linker, test, debugger, or historical Lupos source was used.

Bindings reviewed: Linux `425f94c2954b1fe80ebdbf9b29854e89750355df`;
Phase 0 `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2`;
queue fingerprint `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f`;
scope `b83349e6731e17e5da5e04a0ea053487e8ac8d9269538dbdb367d83f77b17e0a`;
symbols `7484d7b0dd80f45e18d726b04752827fe28555cc5c1af0e127948380e6688abf`;
ABI `ae0caca34fe9b6eb8097958d2fbb8d2b6a1fad60f91f3c2b8d948c43dbfcee39`;
lifetimes `0e7e60940dd21c28f3d10965325f70644fc000495a2d61984437dec666da93d8`.

Result: **FINDINGS**

## Findings

### P01 — incomplete external arrays became zero-length arrays (blocking)

Linux symbols: `compat_write_class`, `compat_read_class`, `compat_dir_class`,
`compat_chattr_class`, and `compat_signal_class`.

Local evidence: the pinned header declares each as `extern unsigned int name[]`
at `include/linux/audit_arch.h:27-31`.  The empty brackets are an incomplete
array declaration: the declaration binds an external object and does not impose
a zero element count.  The candidate instead declares each as `pub static mut
name: [u32; 0]` at `src/include/linux/audit_arch.rs:34-38`.  That gives the Rust
interface a known zero-sized, zero-element array, so every element access is
outside the declared object and the C incomplete-array interface cannot be
represented faithfully through it.  It also records a zero-sized layout where
the Linux declaration intentionally records no bound.  Preserve an external
object/pointer interface that permits the actual defining object's elements;
do not encode an unknown bound as zero.

Affected semantic keys (the two architecture records for each ABI layout):
`SC1-254e9ed195729b0d8c9655371ded405d887cda4579ef36cc9d4d1c04697acbde`,
`SC1-5cc274eae364e917f07a2a70c7d8b33d570b62e16c6367a2b9b8a4b43548cce0`,
`SC1-4e6756a2af60cc3db3dfb6347567ee424d3effedc3bbe23ed6f41a0e66ddca78`,
`SC1-7a95c55ab178c34393a20d1a4e596a099e25196d131f0e2d997afd832fa0365d`,
`SC1-084308fe22ee272dd121e4791ffd6cd6b8d73da163031f5f3bef8feeabe03a1b`,
`SC1-e3e6004c9e8db666eb75a66080a58c12e9e5c6d0b2e6716230e84f86d4c9cd35`,
`SC1-99c5215a75308d396bb8cf843c83bc4acca115bc98cfb36066fe45068c4fa0f7`,
`SC1-e3a0f0b0dc4ddd506d5f2c5fb2c83a2ca73f10c05d8c439e1f5bf43a8e486982`,
`SC1-32fc3afcc975c9921f8ba6e1fd7c82a4b62653d2bbf3477b8b28394a6e2f7f6c`,
and `SC1-8c14b1c87bb06e0d0ce633cf0eb9b1c19568257b68a92f109a94ebc1c55b1dfd`.

### P02 — C enumerator ordinary-namespace/int semantics changed (blocking)

Linux symbols: `AUDITSC_NATIVE`, `AUDITSC_COMPAT`, `AUDITSC_OPEN`,
`AUDITSC_OPENAT`, `AUDITSC_SOCKETCALL`, `AUDITSC_EXECVE`, `AUDITSC_OPENAT2`,
and `AUDITSC_NVALS`.

Local evidence: `include/linux/audit_arch.h:12-21` declares a C `enum
auditsc_class_t`; its eight enumerators are ordinary-identifier integer
constants, with values 0 through 7 (also recorded in frozen `SYMBOLS.tsv` rows
155533-155540 and 155550-155557).  The candidate exposes enum variants and
then re-exports those variants at `src/include/linux/audit_arch.rs:9-28`.
Consequently each bare `AUDITSC_*` expression has Rust enum type
`auditsc_class_t`, not the C enumerator's integer-constant behavior; it also
adds a scoped `auditsc_class_t::AUDITSC_*` interface absent from the C ordinary
namespace.  This changes use in integer expressions, return expressions, and
array-bound/bit-operation contexts.  Preserve the enum object's C layout if
needed, but expose the enumerator values with the C `int` semantics and the
ordinary namespace expected by its consumers.

Affected semantic keys (both architecture selection records for all eight
enumerators): `SC1-2a6f34dd5023a8339e26517e4c1e0ca15e5b8dc5b9b0f37f420d004688fe77fe`,
`SC1-e9bffc7108e280d8e8c069cb938c278197a57bb130567c181c51577cecfab3af`,
`SC1-1b07338eaea1f7041b2462439a7f30f4fbdf1f083b32f7a44d022db78465c6b5`,
`SC1-4d6309bd3dd6d09ab0c004abfdc598b5ba25fecafd339c23afea5449ab51341a`,
`SC1-62c24eabdd2bb37750b748785cc29c5b7482d50ed1950bf990198e664160061b`,
`SC1-242dbcc2fca9ff9540e52fcce715e84384b488c435d6d68c9bed23752e183081`,
`SC1-a94be653ccc21841bc58e1e076cda9ab3143b867904e5292e0d919a5349b8a81`,
`SC1-0dfc2eb52b4f758031969379696a36abfa3c8b4199c677507793364d05e9b900`,
`SC1-b5089bb81ac3c7473e8ef234c9a04a7460f023c6957d698065da7cf92a7399a8`,
`SC1-be5a38fd36a6aa801c2e9f8149da24e82438581cc7dd853b51b0724580d58469`,
`SC1-55706f1aa439ef49c4b6807d1e85ad3c672acbc0c6003044273c053ae1133c77`,
`SC1-db282bfc4403a956632c4b3e8a04388cfba7db3bc4f2c0957ce6cc7f087fe3b1`,
`SC1-12fde02a3235a3389b553a93ceb0b0677e6b60573c894cd705bd429633d81382`,
`SC1-06a311d7c9345183089248a90e64a603ab816e3326cfd72e5d1d0a4eb4a72b32`,
`SC1-8631822cc3384885afdc264a57fd308f25ad788d02e96ed7b879267121346421`,
and `SC1-c83cf7f4f990c94c2cf101d41e271f7dca76f695d8bb4eeb2d23a1aa30093cf6`.

### P03 — declared external function missing from frozen symbol/ABI/lifetime closure (blocking)

Linux symbol: `audit_classify_compat_syscall`.

Local evidence: the pinned header declares `extern int
audit_classify_compat_syscall(int abi, unsigned int syscall);` at
`include/linux/audit_arch.h:24`; the candidate has an apparently corresponding
`extern "C"` declaration at `src/include/linux/audit_arch.rs:32`.  However,
the complete S013482 rows in frozen `SYMBOLS.tsv`, `ABI.tsv`, and
`LIFETIMES.tsv` contain the enum and five arrays but no function record, and
the 161-record sealed proposal has no `audit_classify_compat_syscall` key.
Therefore the proposed `SCOPE.tsv` completion cannot establish the selected
function's linkage, C calling convention, argument/return ABI, or lifetime
contract.  Add this Linux symbol to the authorized semantic inventory/closure
through the required scope process; do not mark this header semantically
complete while it remains untracked.

Affected semantic key: `SC1-caba1fdcd2c1d4dc79a2f57b1c88ecf7125acb2d835030a93c4baddb8069d2db`.

## Audited without a finding

The C include guard at `include/linux/audit_arch.h:9-10,33` has no independent
runtime or linkage behavior once this path is one Rust module, so its absence
as a textual Rust macro is not itself a parity defect.  The candidate preserves
the function's visible C `int, unsigned int -> int` spelling at the source
interface; P03 is the missing frozen inventory/closure evidence, not an
assertion that this particular declaration has the wrong widths.

`AUDIT_ARCH_*` bit composition is defined outside this leased header in
`include/uapi/linux/audit.h:389-451`.  Its relevant x86_64/AArch64 callers
return those values as `int` in `arch/x86/include/asm/syscall.h:167-172` and
`arch/arm64/include/asm/syscall.h:116-120`.  This header only receives `abi`
as `int`; it neither defines nor transforms the `AUDIT_ARCH_*` macros, so no
unrelated UAPI macro translation is claimed or required here.

No unauthorized branding was found.
