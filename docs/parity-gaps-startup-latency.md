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

---

## 9. Desktop services are stuck in `__TASK_STOPPED` — likely why Firefox never opens

**Localized 2026-08-01** with the `lupos.trace=stall` detector
(`src/kernel/idle_stall.rs`), serial `1785588327976480831`.

At the Firefox phase the stall dump shows a large, systematic set of session
services in state `T` (`__TASK_STOPPED`, `0x0004`), `on_rq:0`:

```
xdg-desktop-portal (3628, 3652)   xdg-document-portal (3644)
xdg-permission-store (3638)       at-spi-bus-launcher (3995)
dbus-daemon (4009)                dconf-service (729)
polkit-gnome-authentication (811) gpgconf (4046)
(sd-close) (4206, 4208)           wrapper-2.0 (853)
bwrap (x16)                       glycin-image-rs (x6), glycin-svg (x3)
```

These are precisely the services Firefox needs to finish starting (portals,
at-spi, session dbus, dconf). With all of them stopped, Firefox never reaches a
mapped Navigator window — which matches every observation to date, including
the machine then going fully idle with empty runqueues.

**Not yet root-caused, and deliberately not theorized further.** What is
established:

- The console path does **not** implement job control: `tty_check_change()` /
  `SIGTTOU` exist only in `src/linux_driver_abi/tty/pty.rs`, so the serial
  console did not stop them.
- The stopped set is dominated by `bwrap`-sandboxed helpers (`glycin-*`) and
  by services started under the user session.
- `bwrap --unshare-all` now succeeds (`rc=0`), i.e. the namespace work in
  `4aff0ec` changed behavior in exactly this area.

**Phase 2 target before any further debugging**, per the bug investigation loop
in `AGENTS.md`: bring the group-stop and job-control paths to 1:1 with

- `vendor/linux/kernel/signal.c`: `do_signal_stop()`,
  `task_participate_group_stop()`, `prepare_signal()`, `ptrace_stop()`,
  `SIGNAL_STOP_STOPPED`/`SIGNAL_STOP_CONTINUED` handling;
- `vendor/linux/drivers/tty/tty_jobctrl.c`: `__tty_check_change()`,
  `__proc_set_tty()`, `tty_signal_session_leader()` — including the console,
  which currently has none of it;
- `vendor/linux/kernel/exit.c`: orphaned-pgrp handling
  (`will_become_orphaned_pgrp()`, `kill_orphaned_pgrp()`), which is what is
  supposed to `SIGCONT` a stopped orphaned group.

Only once those match Linux should the remaining stopped-task behavior be
debugged.

---

## 10. `SO_PEERCRED` pid is not namespace-translated; D-Bus sees no peer pid

**File:** `src/net/socket.rs:973` (`current_tgid_vnr`), `src/net/syscalls.rs`
(`copy_unix_peercred`)
**Linux:** `net/core/sock.c:cred_to_ucred()` -> `pid_vnr()`,
`net/unix/af_unix.c:init_peercred()`/`copy_peercred()`

**Verified by reading both sources.** Despite its name, `current_tgid_vnr()`
performs **no namespace translation at all**:

```rust
let tgid = (*task).tgid;
if tgid > 0 { tgid } else { (*task).pid }
```

It never calls `task_tgid_vnr()`/`task_tgid_nr_ns()`, which do exist in
`src/kernel/pid_namespace.rs`. Two divergences follow:

1. Linux translates with `pid_vnr()` **at `getsockopt()` time, into the
   reader's pid namespace**, and yields **0** when the peer is not visible
   there. Lupos captures a raw global tgid at socket-creation time and hands
   that same number to every reader regardless of namespace.
2. That makes the value wrong precisely for `bwrap`-sandboxed peers, which are
   in their own PID namespace — the case the desktop actually exercises.

**Runtime symptom, verified** (serial `1785589995862509642`):

```
dbus-daemon[1936]: [session uid=1000 pid=18446744073709551615 pidfd=5] Activating service name='org.xfce.Xfconf'
```

`18446744073709551615` is `(uint64)-1` — D-Bus's "pid unset". D-Bus obtained a
`pidfd` but **no peer pid**. This matters beyond cosmetics: D-Bus policy and
`xdg-desktop-portal` identify callers by peer pid, and the portals are among
the services that end up stopped/dead (gap 9).

---

## 11. LightDM restart loop is the perceived "slow lightdm init"

**Verified** (serial `1785589995862509642`): `Starting Light Display Manager`
appears **10 times** in one boot, with `Session pid=1052: Exited with return
value 1` nine times. The XFCE session starts, exits 1, LightDM restarts, repeat.

LightDM's own init is **not** slow: from `Starting Light Display Manager` to
`Prompt greeter` is 1.43 s (X server launch +0.03 s, ready signal +0.70 s,
greeter connected +1.09 s). The 1.4 s -> 8.3 s gap in a normal run is the
harness deliberately typing a wrong password first. The user-visible delay is
the restart loop, not initialization.

Contributing evidence from `.xsession-errors` in the same run:

- `Failed to connect to user scope bus via local transport: $DBUS_SESSION_BUS_ADDRESS and $XDG_RUNTIME_DIR not defined`, then `Connection refused`
- `xfwm4-WARNING: Another compositing manager is running on screen 0` — a
  leftover from the previous loop iteration, so each restart poisons the next.

The `desktop-initial-failed` classification here came from the relay detector,
so this mode is no longer a silent six-minute timeout.

---

## 12. The wallpaper is missing because the image loaders are stopped, not because the framebuffer is broken

**Verified** by band-analysis of the host scanout capture
`serial-graphics-x11-1785588327976480831-initial-desktop.ppm` (1280x800):

```
band y=  0- 80: nonblack 1165/3200 (36%)   <- XFCE top panel renders
band y= 80-720: nonblack ~0     (0%)       <- desktop area black
band y=720-800: nonblack  468/3200 (14%)   <- bottom panel renders
```

The panels reach the screen, so `/dev/fb0`, the Xorg `fbdev` driver
(`ShadowFB=false`, direct mmap writes) and the scanout path all work. Only the
wallpaper is absent.

**Correction to an earlier reading in this investigation:** the guest probe
line `initial-framebuffer ... nonzero=1217/256000 percent=0` samples
`band-y=200 rows=200`, which lies entirely inside the black desktop region. It
is *not* evidence that the framebuffer is broken, and was previously read that
way.

Cause is gap 9: the `glycin-image-rs` / `glycin-svg` loaders that decode the
wallpaper are `bwrap`-sandboxed and go **S -> T (`__TASK_STOPPED`) -> X
(`TASK_DEAD`)** — verified across the stall dumps. With no decoder, xfdesktop
paints nothing.

### The group-stop path has no Linux bookkeeping

`src/kernel/signal.rs:stop_current_for_signal()` is:

```rust
(*task).__state.store(__TASK_STOPPED);
wake_waiters(task);
schedule_with_irqs_enabled();
(*task).__state.store(TASK_RUNNING);   // unconditional
```

Linux `kernel/signal.c:do_signal_stop()` instead sets `SIGNAL_STOP_STOPPED`,
counts participants in `signal->group_stop_count` via
`task_participate_group_stop()`, and keeps the task stopped until SIGCONT or
SIGKILL. Lupos has **no** `SIGNAL_STOP_STOPPED`, `SIGNAL_STOP_CONTINUED`, or
`group_stop_count` anywhere (grep returns nothing), and no
`kill_orphaned_pgrp()` to SIGCONT an orphaned stopped group. A task stopped
here is therefore resumable only by an explicit SIGCONT that nothing sends.

This is the Phase 2 target for both gap 9 and this gap.

### Quantified: the LightDM restart loop *is* the long-standing boot bimodality

Measured across today's runs (`greeter-ready elapsed-us` from each harness log):

| run | greeter-ready | desktop-ready |
| --- | --- | --- |
| A (pre-fix) | 8.6 s | wedged, never |
| B, D, E, F, G | 7.8 - 8.5 s | 22.4 - 24.0 s |
| **H (restart loop)** | **107.8 s** | never |

Two conclusions:

1. The fixes landed in `d231d0a` did **not** measurably change time-to-greeter
   (~8 s before and after). What they changed is **reliability**: pre-fix runs
   wedged outright, post-fix runs consistently reach the desktop at ~22.5 s.
   Any claim that "boot latency was fixed" should be stated that way — as a
   removed hang, not a shortened boot.
2. The 107.8 s outlier in run H is the LightDM session restart loop, and it
   matches the previously recorded "~9 s vs ~108 s" boot bimodality almost
   exactly. That bimodality is therefore **not** a scheduler or timer effect:
   it is the XFCE session exiting with status 1 and LightDM restarting ~10
   times, each iteration poisoned by the previous one's leftover compositor.

### Falsified: the stopped tasks are NOT being stopped by a signal

Run I (serial `1785590899893257318`) reached `desktop-initial-ready` and showed
**33 tasks in `state:T (0x0004)`**, while a probe placed inside
`signal.rs:stop_current_for_signal()` fired **zero times**.

The probe is confirmed live: `strings` finds `signal: group-stop` in the kernel
built at 22:28, and the run started at 22:33.

`stop_current_for_signal()` is the only production writer of `__TASK_STOPPED` —
`DefaultAction::Stop` routes through it, and every other `__state.store()` in
the tree writes `TASK_RUNNING`, `TASK_INTERRUPTIBLE`, `TASK_UNINTERRUPTIBLE`,
`__TASK_TRACED`, `TASK_KILLABLE`, `TASK_DEAD`, or `EXIT_ZOMBIE` (checked
exhaustively by grep).

**So no code path writes `0x0004`, yet 33 tasks read `0x0004`.** The
job-control / group-stop hypothesis in gaps 9 and 12 is therefore **wrong as a
cause**, and the missing `SIGNAL_STOP_STOPPED`/`group_stop_count` bookkeeping —
while a real parity gap — is not what produces these T states.

Remaining candidates, in priority order:

1. **`__state` corruption.** This investigation already found two memory-safety
   defects in this area (the original IRQ-entry frame carrying two metadata
   words, and the 8 KiB stack temporary in `begin_task_struct_rcu_release`). A
   third writer clobbering `__state` would look exactly like this.
2. **Stale enumeration.** `idle_stall` walks `for_each_heap_task()`; if freed or
   partially-recycled `task_struct`s are still reachable there, `__state` is
   garbage. Note the same dumps show entries with empty `comm` and `pid 0`.

Next step is a watchpoint on one victim's `__state` (for example
`xdg-desktop-portal` pid 3650) in a symbolized debug/KVM run, per the
GDB-first rule — not another parity edit, because the parity theory has been
falsified.

**A related genuine parity bug found while checking:**
`src/kernel/exit.rs:371` stores `EXIT_ZOMBIE` into `__state`:

```rust
let task_state = if autoreap { TASK_DEAD } else { EXIT_ZOMBIE };
(*tsk).__state.store(task_state, Ordering::Release);
```

Linux keeps `__state` and `exit_state` as **separate fields**
(`include/linux/sched.h`); `exit_notify()` sets `tsk->exit_state = EXIT_ZOMBIE`
and never puts it in `__state`. Conflating them means any `__state` consumer
sees `0x20` for a zombie. This is a real divergence and a plausible source of
further bogus state readings.

---

## 13. FIXED — the sender was writing `__TASK_STOPPED` onto a sleeping target

**File:** `src/kernel/signal.rs` (`apply_remote_stop`, formerly two inline
`if stop_now` blocks)
**Linux:** `kernel/signal.c:prepare_signal()` / `complete_signal()` /
`do_signal_stop()`

Gaps 9 and 12 blamed missing group-stop bookkeeping. That was **wrong**, and the
probe proved it: 33 tasks sat in `state:T (0x0004)` while a probe inside
`stop_current_for_signal()` — believed to be the only writer — fired **zero**
times.

An exhaustive enumeration of every `__state.store()` in the tree (by stored
value, not by same-line grep, which is what hid it the first time) found **five**
writers of `__TASK_STOPPED`. Two of them were in the *signal sender*:

```rust
let stop_now = is_stop_signal(sig) && target != current;
...
if stop_now {
    (*target).m26.ptrace_stop_signal = sig;
    (*target).__state.store(__TASK_STOPPED, Ordering::Release);   // remote write
```

So any stop signal sent to another task overwrote that task's `__state` from
the **sender's** context. Linux never does this: `prepare_signal()` flushes
SIGCONT, leaves the signal pending, and `signal_wake_up()` lets the target stop
**itself** in `get_signal()` -> `do_signal_stop()`.

**Why it was fatal:** if the target was asleep in `TASK_INTERRUPTIBLE` (futex,
poll, read), the store destroyed the sleep state it was waiting in. The event
it was waiting for then arrived as
`wake_task_with_state(target, TASK_INTERRUPTIBLE, ..)`, no longer matched
`__TASK_STOPPED`, and was **lost forever**. The task stayed `T` and the machine
went idle with empty runqueues — the exact signature chased through this whole
investigation.

Fix: `apply_remote_stop()` publishes `__TASK_STOPPED` only for a target that is
already `TASK_RUNNING`; a sleeping target keeps its wait state and is marked
pending + woken so it reaches its own signal-delivery path.

### Verified result (serial `1785604130487843301`)

| metric | before | after |
| --- | --- | --- |
| tasks in `state:T (0x0004)` | 33 | **0** |
| initial-desktop `painted` | 95/422400 (0%) | **46932/422400 (11%)** |
| desktop band y=80-720 non-black | 0% | **4-7% in every band** |

**The wallpaper renders.** The black desktop was never a framebuffer or DRM
problem: the `glycin` image loaders that decode it were being frozen by this
remote write, so nothing ever produced wallpaper pixels.

`glycin`/`bwrap` helpers are now `S` or `X` (normal sleeping / exited) instead
of `T`.

### Still open: the LightDM restart loop

Same run still shows 6 `Starting Light Display Manager` and 4
`Exited with return value 1`, and that run took 160.7 s to `greeter-ready`
(vs ~8 s when no loop occurs). The restart loop is **not** explained by this
fix and remains the outstanding cause of "LightDM takes a while to init".

---

## 11 CORRECTED — there is no LightDM restart loop

Gap 11 above claimed a LightDM restart loop ("10 starts, 9 session exits").
**That was a miscount and is withdrawn.** Those counts came from `grep -c` over
the serial log, which also matches the relay's diagnostic dumps *echoing*
`/var/log/lightdm/lightdm.log` back to the console. The same line therefore
appears many times for a single real event.

Verified on serial `1785604130487843301`:
`grep -oE 'Starting Light Display Manager 1\.32\.0, UID=0 PID=[0-9]+' | sort -u`
yields exactly **one** instance, `PID=425`, and there is exactly **one**
`Exited with return value 1` (the harness's own deliberate wrong-password
attempt, `Authentication complete with return value 7`).

Always dedupe by the embedded PID before counting events in these logs.

### What the slow mode actually is

Deduplicated timeline of the single `lightdm[425]` in the slow run:

```
+0.00s   Logging to /var/log/lightdm/lightdm.log
+25.07s  Adding default seat                      <- 25 s gap
+26.45s  Got signal 10 from process 443 (X ready)
+146.95s Activating VT 7                          <- 120 s gap
+147.49s Greeter connected
+148.12s Greeter start authentication for lupos
+156.43s Session pid=1361: Running command /etc/lightdm/Xsession startxfce4
```

LightDM's own work is trivial. The cost is two waits totalling ~145 s, and the
serial shows what it is waiting on:

```
 35  A start job is running for User Manager for UID 967
 29  A start job is running for User Manager for UID 1000
```

`user@967.service` is the greeter user's `systemd --user`; `user@1000.service`
is the logged-in user's. So **"LightDM takes a while to init" is
`user@.service` startup stalling**, not LightDM initialization, and it matches
the previously recorded user-runtime-dir/user@ boot bimodality.

In the fast mode the same sequence completes in ~8 s total, so this is
bimodal-stall behaviour, not a constant cost. Gap 13's remote-stop fix removed
one lost-wakeup mechanism but evidently not the one behind this stall.

**Next step:** reproduce the slow mode with `lupos.trace=stall` and capture the
task dump *during* the 120 s `user@967.service` wait — the detector fires at 2 s
of full idleness, so the stall dump will name what `systemd --user` is blocked
on.

### `user@.service` stall: what the dumps do and do not show

31 stall dumps were captured during the slow run (`1785604130487843301`):

- **26 of 31** report `runnable=0 runnable_off_rq=0` — the machine is
  *genuinely* idle with nothing runnable during the `user@967.service` wait.
  So the 120 s gap is a **wakeup that never arrives**, not a scheduler failing
  to pick up a queued task.
- 5 report `runnable=1` or `2`.

**Caveat on the detector, important for anyone reading these dumps:**
`idle_stall::report()` walks the task list **twice** — once to compute
`runnable`/`runnable_off_rq`, once to print the per-task lines. State changes
between the passes, so the aggregate counts and the per-task `state:`/`on_rq:`
fields are not a consistent snapshot. Concretely, `systemd-journal` prints as
`state:R on_rq:1` in 26 dumps while only 5 dumps report `runnable > 0`. Do
**not** conclude "a runnable task was on a runqueue while the CPU idled" from
that pairing — it is a two-pass artifact. Making `report()` take one consistent
snapshot is a prerequisite for trusting this output.

Next step therefore remains: find the wakeup that never arrives during
`user@967.service` startup. Gap 13's remote-stop write was one such lost-wakeup
mechanism and is fixed; this is a second, distinct one.

---

## 14. Wallpaper: the untested path is gdk-pixbuf -> glycin **SVG**, out of process

Corrections to earlier entries in this file, all from repeated measurement:

- Gap 12 said the wallpaper renders once the loaders are unfrozen. **Wrong.**
  Run J's "painted=11%" was bright UI content on a black desktop, not a
  wallpaper: its brightness histogram is bimodal (11425 samples near-black,
  1375 at 224-255) with no mid-tones. A real image would show a spread.
- The follow-up "slow-mode timing artifact" reading was also wrong. Frames
  captured by hand over HMP at **t+5 s, t+25 s and t+55 s** after
  `desktop-initial-ready` are **0% non-black** in the desktop area
  (y 80-720), mean brightness 0.7. The wallpaper never appears in the fast
  mode; it is not merely late.

**The desktop background is black in every run measured.**

### What is verified working

- `graphics-x11: wallpaper-decode ok` and `wallpaper-pixels ok` in every run:
  `glycin-thumbnailer` decodes the stock backdrop
  `/usr/share/backgrounds/xfce/xfce-x.svg` into real pixels.
- `pixbuf ok`, `pixbuf-bridge ok`, `pixbuf-bridge-pixels ok`.
- Panels render, so `/dev/fb0`, the fbdev driver and scanout are fine.
- `T`-state tasks are 0 since gap 13's fix, so the loaders are no longer frozen.

### The gap in coverage

Per `xtask/src/lib.rs:5057`, staged `gdk-pixbuf2` (2.44) compiles in only the
ani/bmp/icns/qtif/xpm loaders — **both PNG and SVG decode are delegated to the
out-of-process `glycin` loaders** — and Arch `librsvg` (2.62) ships no
gdk-pixbuf SVG module at all. So a missing SVG pixbuf module is *expected*, not
a staging bug.

The icon theme was deliberately pointed at the **PNG** `AdwaitaLegacy` theme
"so icon loads only ever need glycin's `glycin-image-rs` loader, never the SVG
one". The `pixbuf-bridge` probe therefore exercises the bridge with a **PNG**
source only.

But xfdesktop's compiled-in stock backdrop is **`xfce-x.svg`**. Its render path
is `xfdesktop -> gdk-pixbuf -> glycin-svg` (out of process, bwrap-sandboxed) —
and that is exactly the path **no passing probe covers**. Meanwhile `glycin-svg`
processes churn S -> X in every run.

`wallpaper-decode ok` does **not** cover it either: that probe invokes
`glycin-thumbnailer` directly, not through gdk-pixbuf.

### Next step

Extend the `pixbuf-bridge` probe to run the **SVG** backdrop through
`gdk-pixbuf-pixdata` (the same in-process bridge GTK uses), not just a PNG. If
that fails while `glycin-thumbnailer` on the same file succeeds, the defect is
in the gdk-pixbuf -> glycin-svg bridge, and the black desktop is explained
without any further kernel change. This is a harness/probe change, not a gate
run, and it is the cheapest decisive test remaining.

### FIXED — wallpaper renders once xfdesktop uses a raster backdrop

The hypothesis in gap 14 was correct: the `gdk-pixbuf -> glycin-svg` bridge is
the broken path.

Fix (`xtask/src/lib.rs`): stage an `xfce4-desktop.xml` per account under
`.config/xfce4/xfconf/xfce-perchannel-xml/` pointing xfdesktop at the already
staged JPEG `/usr/share/backgrounds/lupos-login.jpg` instead of letting it fall
back to its compiled-in `xfce-x.svg`. JPEG decodes through `glycin-image-rs` —
the loader `pixbuf-bridge-pixels ok` verifies every run, and the same one the
icon theme was already switched to PNG in order to use.

Verified on serial `1785606276142523109`, in the **fast** mode that previously
never rendered a backdrop (`greeter-ready` 8.19 s, `desktop-ready` 22.82 s):

| metric | before | after |
| --- | --- | --- |
| harness `painted` | 95/422400 (0%) | **422347/422400 (99%)** |
| desktop-area mean brightness | 0.7 | **164.3** |
| brightness distribution | all in bucket 0 | **spread across buckets 2-7** |

The distribution matters more than the percentage. Earlier in this
investigation a "painted=11%" run was mistaken for a rendered wallpaper; its
histogram was bimodal (bucket 0 and bucket 7, no mid-tones), i.e. bright UI on
a black desktop. A real photographic backdrop shows continuous mid-tones, which
is what this capture has. Use the histogram, not a non-black threshold, when
judging these frames.

**This is a harness/image-configuration fix, not a kernel change.** The kernel
side of the image path was already correct: decode, the pixbuf bridge, fbdev
and scanout all reported healthy throughout. What was missing was that
xfdesktop's default backdrop is the one format whose loader path nothing
exercised.

---

## 15. The remaining slow mode: 13 tasks wedged in `TASK_UNINTERRUPTIBLE`

Captured on serial `1785606627872811798` (run N), which entered the slow mode
after `greeter-ready` at 8.12 s and never reached `desktop-initial-ready`:

```
INFO: cpu idle for 415.02s, runnable=0 runnable_off_rq=0
INFO: cpu idle for 435.02s, runnable=0 runnable_off_rq=0
```

Seven minutes of complete idleness with **zero runnable tasks**. In the dump at
that point the task states are:

```
13  state:D   (TASK_UNINTERRUPTIBLE)   <- includes lightdm pid 553
14  state:S   (TASK_INTERRUPTIBLE)
```

Note `A start job is running for User Manager` count is **0** in this run, so
this is a **different** stall point from the `user@.service` wait recorded
earlier — the slow mode is not a single phenomenon.

Thirteen tasks in uninterruptible sleep for 435 s is a kernel-side wait that
never completes: a lock, an I/O completion, or a mutex. It is not a lost
scheduler wakeup of the kind fixed in gap 13, because `D` tasks are not waiting
on a signal-visible wakeup.

The most likely connection is **gap 4**: AHCI/libata, virtio and the DRM module
ABI have no native IRQ wakeup path and deliver completions through
`poll_driver_abi_events()` at the idle-loop chokepoint. A completion that never
arrives leaves its waiter in `D` forever, and every CPU then halts with nothing
runnable — exactly this signature.

**This is where `/proc/<pid>/wchan` (gap 6, still missing) would end the
investigation immediately** by naming the kernel function each `D` task is
blocked in. Implementing it is now the highest-leverage next step, ahead of
further guessing.

### `blocked-at` probe added; the D-state flavour is roughly 1-in-8

`src/kernel/idle_stall.rs` now dumps, for every task in `TASK_UNINTERRUPTIBLE`,
up to 8 kernel-text-looking return addresses from its saved stack:

```
      blocked-at pid:<pid> retaddr:0x<addr>
```

A `D` task is descheduled, so `thread.sp` is stable and its stack quiescent. No
in-kernel symbolization is needed — and none exists — because the addresses are
resolved on the host:

```bash
addr2line -f -C -e target/xtask/cargo-graphics-x11/x86_64-lupos/release/lupos <addr>...
```

This sidesteps gap 6's missing `/proc/<pid>/wchan` for exactly the case that
matters, at a fraction of the cost of an unwinder plus `kallsyms`.

**Not yet fired.** Runs O and P both entered a slow mode but with `D:0`, so the
probe produced nothing. Across this investigation the slow mode has shown at
least three distinct shapes:

1. `user@.service` waits (~120 s gap, `User Manager` job messages)
2. 13 tasks wedged in `D` for 435 s (serial `1785606627872811798`)
3. no `D` tasks, machine mostly idle in ~2 s gaps, simply grinding slowly

Only shape 2 arms this probe, and it appeared once in roughly eight runs. Loop
the gate until `grep -c blocked-at` on the serial is non-zero, then symbolize.
Treating the slow mode as a single bug is what produced several wrong
conclusions earlier in this file.

### Wallpaper fix confirmed across three independent runs

The n=1 caveat is resolved. Runs M, Q1 and Q2 all report an identical
`painted=422347/422400 percent=99`, each in the fast mode:

| run | greeter-ready | desktop-ready | painted |
| --- | --- | --- | --- |
| M  | 8.19 s | 22.82 s | 422347/422400 (99%) |
| Q1 | 8.19 s | 22.68 s | 422347/422400 (99%) |
| Q2 | 8.29 s | 22.89 s | 422347/422400 (99%) |

Before the fix the same metric was `95/422400 (0%)` in every fast-mode run.
