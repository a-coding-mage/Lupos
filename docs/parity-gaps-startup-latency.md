# Parity gaps affecting startup latency and usability

Audit date: 2026-08-01. Revision `10a55d2f0cb4d575be99b5f66757bd3993b03556`
plus the uncommitted fixes noted below.

Scope: divergences from `vendor/linux` that cost **boot time, interactive
latency, or desktop usability**. Ordered by measured or reasoned impact.

Each entry states how it was established. "Verified" means observed in this
tree or in a run from today; "Reported" means it comes from an earlier
investigation and has not been re-measured. Do not quote a Reported item as
fact without re-checking it.

The repository-wide census for context: 1774 files marked
`//! linux-parity: complete`, 558 `partial`, 67 `stub`. `partial` is the norm,
so the file census is not itself a useful ranking — the entries below are the
ones with an established cost.

---

## 1. Fixed-size arrays where Linux moves lists by pointer — FIXED

**Files:** `src/kernel/fork.rs`
**Linux:** `kernel/rcu/tree.c:rcu_do_batch()`,
`kernel/rcu/rcu_segcblist.c:rcu_cblist_flush_enqueue()`

`begin_task_struct_rcu_release()` used `core::ptr::swap` on two
`[*mut TaskStruct; TASK_STRUCT_RCU_BATCH_CAPACITY]` fields (capacity 1024, so
8192 bytes each). `core::ptr::swap::<T>` materializes a `MaybeUninit::<T>`
**stack** temporary — 8 KiB against a 16 KiB release kernel stack
(`KTHREAD_STACK_ORDER = 2`, Linux's x86-64 `THREAD_SIZE`).

**Verified.** Serial `1785584271833621353`:
`#DF Double Fault ... rip=0x2f58b2` symbolizing to
`begin_task_struct_rcu_release`, immediately followed by
`tlb: shootdown stall source=2 target=0` and `source=1 target=0`. The faulting
CPU stops draining its TLB shootdown queue, so every other CPU wedges forever.

Release-only: debug builds use order 4 (64 KiB) and absorb the same temporary.
This is why the whole family of "release-intermittent" stuck-login and
boot-bimodality symptoms was never reproducible under `LUPOS_PROFILE=debug`.

**Fixed** by moving only the live prefix. Linux never copies the payload.

---

## 2. TLB shootdown uses a static matrix instead of Linux's per-CPU call queue

**File:** `src/arch/x86/mm/tlb.rs`
**Linux:** `kernel/smp.c:smp_call_function_many_cond()`,
`__smp_call_single_queue()`, `arch/x86/mm/tlb.c:native_flush_tlb_multi()`

Divergences, all **verified** by reading both sources:

- Lupos publishes into a fixed `TLB_CALL_SLOTS[source][target]` matrix and
  **always** sends an IPI. Linux queues a CSD on the target's `call_single_queue`
  `llist` and sends an IPI **only on the empty-to-non-empty transition**, with
  the target draining the whole queue via `llist_del_all()`.
- `wait_tlb_call()` had no stall detection at all. Linux has
  `csd_lock_wait_toolong()`, which reports an unresponsive target and re-sends
  `arch_send_call_function_single_ipi()`. **Added today** — this is what made
  gap 1 diagnosable instead of a silent four-CPU hang.
- Lupos asserts neither of Linux's two safety preconditions:
  `lockdep_assert_irqs_enabled()` and `WARN_ON_ONCE(!in_task())`.
- **No per-mm `tlb_gen`.** The file header says so outright. Linux uses
  `mm->context.tlb_gen` to skip already-current CPUs and to avoid full flushes;
  Lupos instead does a conservative full local flush whenever a lazy CPU
  returns to its mm (`reactivate_lazy_tlb()`). This is a steady-state cost on
  every context switch, not just an edge case.

**Latency impact:** conservative full flushes on a 4-vCPU desktop workload, plus
an unbounded stall whenever any acknowledgement is missed.

---

## 3. All device interrupts on one shared INTx line, pinned to CPU0

**Files:** `src/linux_driver_abi/pci/msi/*`, `src/kernel/irq/*`
**Linux:** `drivers/pci/msi/*`, `kernel/irq/manage.c` affinity

**Verified** from today's run (`graphics-x11: irq`, serial
`1785585313945078787`):

```
irq  1:      50  -  i8042
irq 11:   43398  -  snd_hda_intel:snd_hda_intel, virtio0, virtio1
irq 12:       0  -  i8042
irq 15:       8  -
```

- Audio and **both** virtio devices share INTx line 11, taking 43k interrupts.
  Every one requires the shared-handler chain to poll each registered device.
- The table has a **single `CPU0` column**: every device interrupt lands on
  CPU0 regardless of `LUPOS_QEMU_SMP=4`. No IRQ affinity, no spreading.
- MSI/MSI-X entry points exist (`pci_enable_msi`, `pci_enable_msix_range` in
  `src/linux_driver_abi/pci/msi/api.rs`) but nothing in the running system uses
  them — the observed distribution is pure shared INTx.

This is the single largest structural interactive-latency gap: audio and disk
and network contend on one line on one CPU, under exactly the workload
(desktop + browser + audio) the project is targeting.

---

## 4. Driver completions are polled at idle instead of interrupt-driven

**File:** `src/kernel/sched/idle.rs` (`pump_driver_abi_events_on_idle()`)
**Linux:** ordinary per-device IRQ handlers; there is no counterpart.

The code comments state this plainly and are worth quoting as the definition of
the gap: it is a "Lupos-specific bridge with no Linux counterpart, because Linux
does not need one: several Linux-built drivers here (AHCI/libata, virtio, the
DRM module ABI) have no native IRQ wakeup path yet and deliver their
completions through `poll_driver_abi_events()` instead."

**Verified** by reading the source. Consequence: I/O completion latency is
bounded by when some CPU next reaches the idle loop or a cooperative
`schedule_with_irqs_enabled()` chokepoint, not by the device interrupt. On a
loaded system that is unbounded.

---

## 5. DRM/GPU is a stub — no KMS, no acceleration

**Files:** `src/linux_driver_abi/gpu/drm/mod.rs`, `src/rust/helpers/drm.rs`,
`src/rust/helpers/gpu.rs` (all `//! linux-parity: stub`)

**Verified** from the parity headers. The desktop therefore runs on the GRUB
VBE linear framebuffer via fbdev with software rendering only. Every desktop
composite, scroll, and video frame is CPU work. Directly responsible for the
"no lag" goal being unreachable for video playback, independent of the
scheduler.

Related **verified** symptom from today's runs: the captured initial-desktop
frame is `painted=95/422400 (0%)` — the black-frame issue that
`LUPOS_ALLOW_BLACK_FRAME=1` currently masks.

---

## 6. `/proc` reported constants instead of real state — FIXED (partly)

**Files:** `src/fs/proc/base.rs`, `src/fs/proc/array.rs`
**Linux:** `fs/proc/array.c:task_state_array[]`,
`include/linux/sched.h:__task_state_index()`

`/proc/<pid>/status` **and** `/proc/<pid>/stat` (what `ps` reads) returned
`R (running)` for every non-zombie task. No sleeping task was observable from
userspace. **Verified**, and **fixed** today — `ps` in-guest now reports
`S`, `Ss+`, `S+`, `R`.

This is not merely a missing feature: it invalidated every prior
"`TASK_RUNNING`" observation in the investigation notes, including the
conclusion that dbus-broker/LightDM/Xorg were runnable during a desktop stall.

**Still missing:** `/proc/<pid>/wchan` does not exist at all — the graphics
probe logs `No such file or directory`, which is why every Lupos thread dump
shows `wchan=` empty. `__get_wchan` exists in
`src/arch/x86/kernel/process.rs` but has **no callers**, performs no unwind,
and there is no `kallsyms` for `%ps` symbolization. Linux:
`fs/proc/base.c:proc_pid_wchan()` + `arch/x86/kernel/process.c:__get_wchan()`.
Without it, a sleeping task's blocking function is invisible — this is the
main reason the remaining Firefox stall is still unresolved.

---

## 7. seccomp user notification is unimplemented

**File:** `src/kernel/seccomp.rs`
**Linux:** `kernel/seccomp.c` `SECCOMP_RET_USER_NOTIF` handling,
`SECCOMP_IOCTL_NOTIF_RECV` / `_SEND` / `_ID_VALID`

**Verified:** `seccomp_action_to_result()` maps
`SECCOMP_RET_TRACE | SECCOMP_RET_USER_NOTIF => SeccompCheck::Errno(-ENOSYS)`.
For `SECCOMP_RET_TRACE` with no tracer that matches Linux; for
`SECCOMP_RET_USER_NOTIF` it does **not** — Linux blocks the caller and forwards
the syscall to the supervisor over the listener fd.
`SECCOMP_FILTER_FLAG_NEW_LISTENER` is defined but no notify ioctl exists.

Affects sandboxed applications, which is precisely the Firefox content-process
model.

### Note on the `enosys` serial counts

Earlier notes treat `enosys pid= nr=` lines as missing syscalls. They are not:
the log site in `syscall_dispatch_ptregs_inner()` fires on **any** final
`-ENOSYS`, which includes a seccomp `SECCOMP_RET_ERRNO(ENOSYS)` and the
`USER_NOTIF`/`TRACE` mapping above. All of the high-count numbers observed
(`321` bpf, `435` clone3, `29` shmget, `443` quotactl_fd, `334` rseq,
`204` sched_getaffinity) **are present** in `SYS_CALL_TABLE`. Do not cite these
counts as unimplemented syscalls without first separating the seccomp path.

---

## 8. No release profile tuning

**File:** `Cargo.toml`

**Verified:** there is no `[profile.release]` section at all, so the kernel is
built with cargo defaults — no `lto`, no `codegen-units` control, no
`panic` strategy choice. Linux builds with `-O2` plus LTO options and an
explicitly tuned pipeline. This is a whole-kernel constant factor on every
path measured above.

---

## Reported but not re-verified in this audit

Carry these forward as leads, not findings:

- `/proc/interrupts` was described as a stub; the module exists
  (`src/fs/proc/interrupts.rs`) and produced the real table quoted in gap 3, so
  that description is at least partly stale.
- Module `/proc` entries invisible: `proc_create`/`proc_mkdir` from modules
  reportedly push into a private vec, so `/proc/asound` never appears.
- LightDM exiting cleanly between `Registering session with bus path` and
  `Running command /etc/lightdm/Xsession startxfce4` — mechanism still unknown.

---

## Top open defect

At a reproduced Firefox stall (serial `1785585313945078787`, silent >430 s), all
four vCPUs were `HLT=1`, `sched::rq::NR_RUNNING_SNAPSHOT` was **all zero**, and
`JIFFIES` plus all four `PER_CPU_TIMER_TICKS` were advancing. Timers fire, time
moves, every runqueue is empty, and no task ever becomes runnable again. The
defect is in the timer-expiry / I/O-completion wakeup path, not in scheduler
placement. Gaps 4 and 6 are the two that most directly obstruct finishing it.
