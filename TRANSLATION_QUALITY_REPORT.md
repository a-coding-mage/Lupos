# C→Rust Translation Quality Report

**Scope:** all 2,401 Rust source files under `src/`, each compared against the
Linux C source its `//! linux-source:` header names.
**Baseline:** `torvalds/linux` tag **v6.19**, with 68 files backfilled from
mainline master for references newer than the tag.
**Companion data:** `reports/translation-analysis/file_verdicts.tsv` carries the
per-file verdict, coverage estimate, pattern codes, and notes for every file.

---

## 1. Executive summary

Lupos's translation process is very good at two things: **mirroring data**
(constants, `#[repr(C)]` structs, tables — nearly always byte-accurate) and
**extracting pure algorithms** (parsers, checksums, schedulers' math, crypto
cores — usually faithful and often more idiomatic than the C). It is
systematically weak at one thing: **being honest about what was left behind.**
The kernel-integration layer — locking, sk_buff/VFS plumbing, module
registration, interrupt dispatch, real I/O — is the part most often missing,
and the file headers rarely say so.

The single most important number in this report:

> Of the 1,620 non-index files whose header claims `linux-parity: complete`,
> only **588 (36%) are full translations**. **642 (40%) are shells** — constant
> mirrors, struct shapes, or errno stubs with none of the C file's operative
> logic.

By contrast, the 528 `partial` headers and 66 `stub` headers are almost
perfectly honest (272/272 non-index `partial` files verified PARTIAL or
better; 64/66 `stub` files verified STUB). **The dishonesty is concentrated
entirely in the `complete` tier.** Fixing how "complete" is earned is the
highest-leverage change available.

The second most important finding: **1,159 files (48% of the tree) "test"
themselves by `include_str!`-ing the vendored C file and asserting substrings
of C source text.** These tests pin the vendor tree against drift — useful —
but they exercise zero Rust behavior, while the `test-origin: linux:...`
header (present on 2,057 files) advertises them as Linux-derived tests. This
is parity theater: it makes the dashboard green without making the kernel
correct.

## 2. Methodology

- Every file's `linux-parity:` / `linux-source:` headers were extracted and
  resolved against a local Linux v6.19 tree (`vendor/linux`, gitignored).
  2,312 resolved directly; 68 resolved on mainline master and were
  backfilled; 2 resolve nowhere (see §6.5).
- The tree was split into 52 batches (balanced by line count, grouped by
  directory). One analysis agent per batch compared each Rust file against
  its C counterpart's function inventory and judged: a **verdict**, a
  **claim-honesty check** against the header, a rough **coverage %** of C
  functions with real Rust logic, and **pattern codes**. Six files the
  agents missed were analyzed by hand.
- Verdict vocabulary: `FULL` (substantially all C logic), `PARTIAL` (real
  logic, meaningful subset), `SHELL` (constants/struct shapes/pure helpers
  only), `CONSTS` (literally constants), `STUB` (errno stubs), `NOSRC`
  (cited C source doesn't exist), `INDEX` (mod.rs/aggregator).
- Limitations: single-pass review; large C files were sampled rather than
  read exhaustively; coverage percentages are estimates, not measurements.
  Treat per-file rows as strong leads, not verdicts of record.

## 3. Repo-wide statistics

**Inventory:** 2,401 files, 586,039 lines of Rust, 26,383 functions, 6,579
`#[test]`s, ~17,400 lines mentioning `unsafe`, 240 files with `#[repr(C)]`.

**Verdicts (all 2,401 files):**

| Verdict | Files | Share |
|---|---|---|
| PARTIAL | 635 | 26.4% |
| FULL | 596 | 24.8% |
| SHELL | 479 | 20.0% |
| INDEX (aggregators) | 400 | 16.7% |
| CONSTS | 161 | 6.7% |
| STUB | 97 | 4.0% |
| NOSRC | 32 | 1.3% |
| DIVERGENT | 1 | <0.1% |

**Header honesty:** 1,721 ok · **577 overclaim (24%)** · 77 underclaim.
Mean estimated coverage of the C counterpart's functions: **37%**; 1,128
files sit at ≤20% coverage while 509 sit at ≥80%.

**Pattern frequency** (multiple per file): pure-logic extraction P3 ×1041,
faithful translation P4 ×850, data-shape mirror P2 ×813, consts-only P1
×652, idiomatic win P5 ×461, source-text tests P7 ×433 (agents tagged it
only when dominant — a direct grep finds `include_str!` of vendored C in
1,159 files), C-ism carryover P6 ×177, errno-stub policy P8 ×80, silent
subset P11 ×74, stale reference P10 ×21, divergence risk P9 ×7.

## 4. Where the tree is strong and where it is façade

Subsystems where the translation is real (FULL-dominant):

| Subsystem | FULL / analyzed | Note |
|---|---|---|
| kernel/sched | 29/40 | control flow tracks C closely, Result error paths |
| kernel/time | 23/35 | |
| kernel/irq | 20/26 | |
| kernel/locking | 18/27 | RAII guards; but see §6.1 on "complete" claims |
| lib/crc | 19/22 | table + algorithm ports, behavior-tested |
| lib/crypto | 33/50 | const-fn table generation, KAT vectors |
| fs/proc | 18/34 | |
| security/integrity | 12/18 | |
| lib (top level) | 56/107 | rbtree, min_heap, sort are model translations |
| arch/x86 (mixed) | 152/560 | boot/ and cpu/mce excellent; events/ and platform/ mostly shells |

Subsystems that are largely surface (SHELL/CONSTS-dominant, most while
claiming `complete`):

| Subsystem | SHELL+CONSTS / analyzed | What's actually missing |
|---|---|---|
| kernel/trace | 63/86 | event definitions mirrored; no tracing engine |
| rust/helpers | 56/61 | metadata contracts for C shims; no logic at all |
| net/dsa | 23/25 | tag math only; no sk_buff xmit/rcv plumbing |
| net/ipv4, net/ipv6 | 20/23, 17/19 | header constants + pure helpers; no stack wiring |
| net/netfilter | 25/65 SHELL, 38 PARTIAL, **0 FULL** | match/target decision logic without xt registration or skb context |
| lib/test_fortify | 20/21 | trivially mirrored test scaffolding |
| kernel/bpf | 10/23 SHELL, 0 FULL | BTF parsing real; verifier/runtime absent |
| fs/smb, fs/xfs | 11/19, 9/15 | struct shapes; no I/O paths |

The `net/` tree deserves emphasis: **not a single file under net/netfilter,
net/ipv4, net/ipv6, net/dsa, or net/6lowpan earned FULL**, yet the majority
carry `linux-parity: complete`. The networking that actually works in Lupos
lives elsewhere (e.g. `src/net/socket.rs`, `src/net/device.rs` are honest
PARTIALs); the per-C-file mirrors are decorative.

## 5. What the translation process does well — keep doing these

These patterns appeared repeatedly in FULL-verdict files and should be the
house style:

1. **Trait seams for C indirection.** C function-pointer tables and
   platform hooks become traits (`A20Platform`, `BiosCaller`, `PortIoOps`,
   `KernelVsyscallEnv`, `RomMemory`). This preserves the C structure, gains
   type safety, and makes host-side testing possible. Best-in-class:
   `src/arch/x86/boot/a20.rs`, `src/arch/x86/entry/vsyscall/vsyscall_64.rs`.
2. **`#[repr(C)]` mirrors with line-cited fields and size/offset
   assertions.** `src/arch/x86/kernel/static_call.rs` asserts
   `size_of == 8`; boot structs cite Linux line numbers per field. This is
   the right way to do ABI parity.
3. **Result/Option/enums over errno and sentinels.** `SeverityLevel` and
   `SmcaBankType` enums in mce, `TscSyncResult`, `FredAction` dispatch,
   `ScriptOutcome` in binfmt_script — all preserve C behavior while making
   invalid states unrepresentable.
4. **`const fn` table generation.** 35+ files in lib/crypto and lib/crc
   build lookup tables at compile time instead of copying C's generated
   statics — provably equivalent and less error-prone.
5. **RAII over manual cleanup.** `MutexGuard`, `Drop` for registration
   handles (`StaticCallTrampKeyRegistration`), owned buffers over borrowed
   raw pointers.
6. **Real vector consumption.** `src/lib/math/test_mul_u64_u64_div_u64.rs`
   ports Linux's actual test vectors — this is what `test-origin: linux`
   should mean everywhere.
7. **Honest scope headers where they exist.** The microcode/SGX/resctrl
   stubs and `src/mm/madvise.rs`-style milestone notes show the project
   already knows how to declare scope; it just doesn't do it consistently.

## 6. Anti-pattern catalog — what to fix, with the rule that prevents it

### 6.1 `complete` on shells (577 files overclaim; 40% of `complete` tier)
A DSA tag driver with tag constants and a pure `const fn` encode/decode but
no sk_buff path claims `complete` (`src/net/dsa/tag_mxl_gsw1xx.rs`).
`src/kernel/locking/mutex.rs` claims `complete` at 662 lines against C's
1,192 — it's a good RAII mutex, but it is not Linux's mutex (no optimistic
spinning, no wait-list handoff). `src/arch/x86/kernel/kvm.rs` claims
`complete` and is constants-only.
**Rule:** `complete` must be earned by function-inventory diff: every
exported C function has a Rust implementation with real logic, or the file
lists the exceptions. Anything else is `partial` with a scope note.
Suggested mechanical gate: a CI script that counts C functions (ctags) vs
Rust `fn`s with bodies and fails `complete` headers below a threshold.

### 6.2 Source-text tests presented as parity tests (1,159 files)
`assert!(source.contains("skb_push(skb, GSW1XX_HEADER_LEN);"))` proves the
vendored C hasn't changed. It does not prove the Rust does what that line
does. Worse, these tests satisfy the AGENTS.MD instruction to "consume the
already field-proven tests from vendor/linux" in letter but not in spirit.
**Rule:** keep at most one `source-pin` test per file (clearly named
`vendor_source_pin`), and require at least one behavioral test per public
function: same inputs, C-documented outputs (errno values, boundary cases,
byte layouts). Where Linux ships real test vectors (KUnit, crypto testmgr,
lib/math), port the vectors — several files already prove this works.

### 6.3 Silent subsets behind seams and "plan" structs
Files model kernel operations as traits or emit `XyzPlan`/`XyzReport`
decision structs, then test only against mocks — no production backend
exists. Examples: `src/arch/x86/coco/sev/core.rs` (VMGEXIT dispatch exists
only under test), `src/fs/v9fs/fid.rs` (675 lines, no fid lifecycle calls),
`src/io_uring/fdinfo.rs` (renders 9 of the C file's fields, hardcodes
`CqOverflow: 0`, claims complete).
**Rule:** a seam trait without a wired production implementation caps the
file at `partial`. Naming a struct `*Plan` is a self-admission — grep for it
in review.

### 6.4 Errno-as-i32 everywhere, no shared error type (745 files)
`Result<T, i32>` with raw `-EINVAL` literals is the dominant error idiom;
`src/fs/timerfd.rs` returns a bare `512` (ERESTARTSYS). There is no shared
`Errno` newtype in the tree at all.
**Rule:** introduce one `Errno` type (repr-compatible with i32 for ABI
boundaries) in a core crate module; convert at FFI edges only. This is a
mechanical, high-value refactor and removes an entire class of
wrong-sign/wrong-code bugs.

### 6.5 Reference hygiene
The vendor tree the headers reference is a **moving mainline snapshot**, not
a pinned release: 68 `linux-source` paths exist only on master (post-v6.19),
and 2 exist nowhere (`fs/ntfs/quota.c`; `lib/raid/xor/arm/xor-neon.c` — an
invented `lib/raid/` hierarchy for what upstream keeps in `arch/*/lib/`).
21 files were flagged P10 (stale/renamed reference).
**Rule:** commit a `vendor/linux.SHA` pin file, and add a CI check that
every `linux-source:` path exists in the pinned tree. Both failures found
here would have been caught instantly.

### 6.6 Unsafe without contract (94 SAFETY comments for ~17k unsafe lines)
The best files (boot, mce) justify every unsafe block citing the Linux
origin. Most others don't. Hard-coded struct offsets
(`src/fs/char_dev.rs`'s `LINUX_CDEV_OPS_OFFSET = 72`) fail silently when
layouts move.
**Rule:** every `unsafe` block gets a `// SAFETY:` comment; every
hand-written offset becomes `core::mem::offset_of!` or gets a compile-time
assertion.

### 6.7 Genuine divergence risks found (spot-check before trusting)
The per-file risk column in the data appendix lists 481 flagged risks; the
ones most worth verifying first:
- `src/net/mptcp/fastopen.rs` — C wraps state mutation in
  `mptcp_data_lock`; the Rust version takes no lock (flagged race).
- `src/io_uring/fdinfo.rs` — `CqOverflow` hardcoded to 0.
- `src/lib/vsprintf.rs` — fewer than half of Linux's format specifiers;
  unsupported ones silently dropped.
- `src/arch/x86/kernel/amd_nb.rs` — `linux_amd_nb_num()` returns a
  hardcoded 0 rather than the northbridge count.
- `src/fs/ext4/dir.rs` — linear scan where C uses htree (documented, but a
  behavioral difference under load).
- `src/fs/dcache.rs` — BTreeMap cache in place of Linux's hash+LRU while
  claiming complete.

## 7. Process changes, in priority order

1. **Split the parity header into three axes** so a file can be honest in
   one line:
   `//! linux-parity: abi=complete logic=partial integration=stub`
   (surface/data parity vs. algorithm parity vs. kernel wiring). The
   current single `complete` collapses these and is where all the
   inflation hides (§6.1).
2. **Gate `complete` on a function-inventory diff** (mechanical CI check
   against the pinned vendor tree).
3. **Rename source-text tests to `vendor_source_pin` and stop counting
   them as tests of the translation.** Require one behavioral test per
   public function; port Linux's real vectors where they exist (§6.2).
4. **Introduce the shared `Errno` type** and migrate `Result<T, i32>`
   call-sites mechanically (§6.4).
5. **Pin `vendor/linux` to a SHA and validate every `linux-source:` path
   in CI** (§6.5).
6. **Adopt the explicit-subset marker** for every PARTIAL file:
   `//! subset: <implemented> ; missing: <C functions not ported>` — the
   batch analyses repeatedly found that files which already do this
   (madvise, microcode, ext4/dir) were the easiest to trust.
7. **Make `*Plan`/mock-only seams a review flag** (§6.3), and require
   `// SAFETY:` on unsafe blocks (§6.6).
8. **Fix the 577 overclaiming headers.** The data file
   `reports/translation-analysis/file_verdicts.tsv` (`claim_check=over`)
   is a ready-made worklist, sorted worst-first by `cover_pct`.

## 8. Data appendix

- `reports/translation-analysis/file_verdicts.tsv` — one row per Rust file:
  parity claim, C source, verdict, claim check, coverage estimate, pattern
  codes, note, risk, and best-idiom callout.
- Verdict/pattern vocabulary: §2 and §3 above.
- Baseline: Linux v6.19 (git tag), plus master backfills for 68 post-6.19
  references, fetched 2026-07-19.
