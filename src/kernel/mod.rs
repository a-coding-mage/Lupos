//! linux-parity: partial
//! linux-source: vendor/linux/kernel
//! Architecture-independent kernel core: deferred work, scheduling primitives,
//! and process model.
//!
//! | Module     | Milestone | Description                                 |
//! |------------|-----------|---------------------------------------------|
//! | `softirq`  | M6        | Deferred interrupt / tasklet layer          |
//! | `task`     | M20       | `TaskStruct` — Linux-ABI process descriptor |
//! | `thread`   | M20/M21   | `ThreadStruct` — arch-specific CPU state    |
//! | `sched`    | M21/M22   | Cooperative scheduler, `kthread_create`     |
//! | `pid`      | M22       | PID bitmap allocator, `struct KPid`         |
//! | `kthread`  | M22       | `kthread_run`, `kthread_stop`, lifecycle    |
//! | `fork`     | M23       | `copy_process`, `kernel_clone`              |
//! | `clone`    | M23       | `CloneArgs`, `CLONE_*`, syscall handlers    |

pub mod audit; // M64
pub mod auditfilter; // M64
pub mod backtracetest;
pub mod bounds;
pub mod configs;
pub mod console;
pub mod crash_core_test;
pub mod debug_trace;
pub mod elfcorehdr;
pub mod softirq;
pub mod static_call;
pub mod syscalls;
pub mod sysctl_test;
// M27 / M28 must be declared before `task` because `task` re-exports
// `cred::Cred` and `nsproxy::Nsproxy` and embeds `seccomp::Seccomp` inline.
pub mod bpf; // M27
pub mod capability; // M27
pub mod cfi;
pub mod cgroup; // M32
pub mod clone; // M23
pub mod cpuhotplug;
pub mod cred; // M27
pub mod dma; // M55
pub mod entry;
pub mod events; // M63
pub mod exec; // M24
pub mod exec_domain;
pub mod exit; // M26
pub mod files; // M39 (task_struct.files glue)
pub mod fork; // M23
pub mod futex; // M32
pub mod gcov;
pub mod groups; // M27a
pub mod irq; // M37
pub mod kconfig; // M65
pub mod kheaders;
pub mod ksyms_common;
pub mod kthread; // M22
pub mod kunit; // M91
pub mod livepatch;
pub mod liveupdate;
pub mod locking; // M33
pub mod module; // M56
pub mod module_signature;
pub mod nsproxy; // M28
pub mod params;
pub mod pid; // M22
pub mod pid_namespace; // M28
pub mod power;
pub mod printk; // M61
pub mod ptrace; // M26
pub mod rcu; // M34
pub mod regset;
pub mod resource_kunit;
pub mod sched;
pub mod seccomp; // M27
pub mod session; // M68/M69
pub mod signal; // M25
pub mod task;
pub mod task_work; // M28a
pub mod taskstats;
pub mod thread;
pub mod time; // M36
pub mod trace; // M62
pub mod ucount; // M27a
pub mod up;
pub mod user; // M27a
pub mod user_namespace; // M28
pub mod user_return_notifier;
pub mod utsname; // M28
pub mod wait; // M26
pub mod watchdog;
pub mod watchdog_buddy;
pub mod workqueue; // M35 // M41
