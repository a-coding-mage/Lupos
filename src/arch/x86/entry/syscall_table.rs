//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry
//! linux-source: vendor/linux/arch/x86/entry/syscall_64.c
//! test-origin: linux:vendor/linux/arch/x86/entry
//! x86-64 syscall dispatch table — 472 entries (0..=471).
//!
//! Verified 1:1 against `syscall_wrappers.rs`: all 370 defined `__x64_sys_*`
//! wrappers are wired, and every wired slot resolves to a defined wrapper.
//! Unimplemented slots default to `sys_ni_syscall` (-ENOSYS), exactly mirroring
//! Linux's `COND_SYSCALL`/`x64_sys_call` default arm in syscall_64.c.
//!
//! Mirrors `vendor/linux/arch/x86/entry/syscalls/syscall_64.tbl` ordering.
//! Each slot is either a real wrapper (from `syscall_wrappers.rs`) or
//! `sys_ni_syscall` (-ENOSYS).
//!
//! Built at compile time via `const fn` — every slot defaults to
//! `sys_ni_syscall`, then we patch in the real entries for syscalls that
//! Lupos currently implements.  As more milestones land, append entries here.
//!
//! Ref: vendor/linux/arch/x86/include/asm/syscall.h::sys_call_table

use super::sys_ni::sys_ni_syscall;
use super::syscall_wrappers as w;
use crate::arch::x86::kernel::ptrace::PtRegs;

pub type SyscallFn = unsafe extern "C" fn(*mut PtRegs) -> i64;

/// Highest implemented common-ABI syscall number plus one.
/// Linux's `__NR_syscalls` for x86-64 — covers numbers 0..=471 inclusive.
/// (x32-only entries 512.. are not part of this table.)
#[allow(non_upper_case_globals)]
pub const NR_syscalls: usize = 472;

/// Build the syscall table at compile time.  Every slot starts as
/// `sys_ni_syscall`; named slots get patched to their real wrapper.
const fn build_table() -> [SyscallFn; NR_syscalls] {
    let mut t: [SyscallFn; NR_syscalls] = [sys_ni_syscall; NR_syscalls];

    // ── fs/read_write/open (rootfs bring-up) ──
    t[0] = w::__x64_sys_read;
    t[1] = w::__x64_sys_write;
    t[2] = w::__x64_sys_open;
    t[3] = w::__x64_sys_close;
    t[4] = w::__x64_sys_stat;
    t[5] = w::__x64_sys_fstat;
    t[6] = w::__x64_sys_lstat;
    t[7] = w::__x64_sys_poll;
    t[8] = w::__x64_sys_lseek;
    t[9] = w::__x64_sys_mmap;
    t[10] = w::__x64_sys_mprotect;
    t[11] = w::__x64_sys_munmap;
    t[12] = w::__x64_sys_brk;
    t[16] = w::__x64_sys_ioctl;
    t[17] = w::__x64_sys_pread64;
    t[18] = w::__x64_sys_pwrite64;
    t[19] = w::__x64_sys_readv;
    t[20] = w::__x64_sys_writev;
    t[21] = w::__x64_sys_access;
    t[22] = w::__x64_sys_pipe;
    t[23] = w::__x64_sys_select;
    t[26] = w::__x64_sys_msync;
    t[27] = w::__x64_sys_mincore;
    t[28] = w::__x64_sys_madvise;
    t[29] = w::__x64_sys_shmget;
    t[30] = w::__x64_sys_shmat;
    t[31] = w::__x64_sys_shmctl;
    t[24] = w::__x64_sys_sched_yield;
    t[25] = w::__x64_sys_mremap;
    t[32] = w::__x64_sys_dup;
    t[33] = w::__x64_sys_dup2;
    t[34] = w::__x64_sys_pause;
    t[35] = w::__x64_sys_nanosleep;
    t[36] = w::__x64_sys_getitimer;
    t[37] = w::__x64_sys_alarm;
    t[38] = w::__x64_sys_setitimer;
    t[39] = w::__x64_sys_getpid;
    t[40] = w::__x64_sys_sendfile;
    t[41] = w::__x64_sys_socket;
    t[42] = w::__x64_sys_connect;
    t[43] = w::__x64_sys_accept;
    t[44] = w::__x64_sys_sendto;
    t[45] = w::__x64_sys_recvfrom;
    t[46] = w::__x64_sys_sendmsg;
    t[47] = w::__x64_sys_recvmsg;
    t[48] = w::__x64_sys_shutdown;
    t[49] = w::__x64_sys_bind;
    t[50] = w::__x64_sys_listen;
    t[51] = w::__x64_sys_getsockname;
    t[52] = w::__x64_sys_getpeername;
    t[53] = w::__x64_sys_socketpair;
    t[54] = w::__x64_sys_setsockopt;
    t[55] = w::__x64_sys_getsockopt;
    t[62] = w::__x64_sys_kill;
    t[64] = w::__x64_sys_semget;
    t[65] = w::__x64_sys_semop;
    t[66] = w::__x64_sys_semctl;
    t[67] = w::__x64_sys_shmdt;
    t[68] = w::__x64_sys_msgget;
    t[69] = w::__x64_sys_msgsnd;
    t[70] = w::__x64_sys_msgrcv;
    t[71] = w::__x64_sys_msgctl;
    t[74] = w::__x64_sys_fsync;
    t[72] = w::__x64_sys_fcntl;
    t[73] = w::__x64_sys_flock;
    t[75] = w::__x64_sys_fdatasync;
    t[76] = w::__x64_sys_truncate;
    t[77] = w::__x64_sys_ftruncate;
    t[78] = w::__x64_sys_getdents;
    t[79] = w::__x64_sys_getcwd;
    t[80] = w::__x64_sys_chdir;
    t[81] = w::__x64_sys_fchdir;
    t[82] = w::__x64_sys_rename;
    t[83] = w::__x64_sys_mkdir;
    t[84] = w::__x64_sys_rmdir;
    t[85] = w::__x64_sys_creat;
    t[86] = w::__x64_sys_link;
    t[87] = w::__x64_sys_unlink;
    t[88] = w::__x64_sys_symlink;
    t[89] = w::__x64_sys_readlink;
    t[90] = w::__x64_sys_chmod;
    t[91] = w::__x64_sys_fchmod;
    t[92] = w::__x64_sys_chown;
    t[93] = w::__x64_sys_fchown;
    t[94] = w::__x64_sys_lchown;
    t[95] = w::__x64_sys_umask;
    t[96] = w::__x64_sys_gettimeofday;
    t[97] = w::__x64_sys_getrlimit;
    t[98] = w::__x64_sys_getrusage;
    t[99] = w::__x64_sys_sysinfo;
    t[100] = w::__x64_sys_times;
    t[102] = w::__x64_sys_getuid;
    t[103] = w::__x64_sys_syslog;
    t[104] = w::__x64_sys_getgid;
    t[105] = w::__x64_sys_setuid;
    t[106] = w::__x64_sys_setgid;
    t[107] = w::__x64_sys_geteuid;
    t[108] = w::__x64_sys_getegid;
    t[109] = w::__x64_sys_setpgid;
    t[110] = w::__x64_sys_getppid;
    t[111] = w::__x64_sys_getpgrp;
    t[112] = w::__x64_sys_setsid;
    t[113] = w::__x64_sys_setreuid;
    t[114] = w::__x64_sys_setregid;
    t[115] = w::__x64_sys_getgroups;
    t[116] = w::__x64_sys_setgroups;
    t[117] = w::__x64_sys_setresuid;
    t[118] = w::__x64_sys_getresuid;
    t[119] = w::__x64_sys_setresgid;
    t[120] = w::__x64_sys_getresgid;
    t[121] = w::__x64_sys_getpgid;
    t[122] = w::__x64_sys_setfsuid;
    t[123] = w::__x64_sys_setfsgid;
    t[124] = w::__x64_sys_getsid;
    t[125] = w::__x64_sys_capget;
    t[126] = w::__x64_sys_capset;
    t[133] = w::__x64_sys_mknod;
    t[136] = w::__x64_sys_ustat;
    t[137] = w::__x64_sys_statfs;
    t[135] = w::__x64_sys_personality;
    t[138] = w::__x64_sys_fstatfs;
    t[140] = w::__x64_sys_getpriority;
    t[141] = w::__x64_sys_setpriority;
    t[142] = w::__x64_sys_sched_setparam;
    t[143] = w::__x64_sys_sched_getparam;
    t[144] = w::__x64_sys_sched_setscheduler;
    t[145] = w::__x64_sys_sched_getscheduler;
    t[146] = w::__x64_sys_sched_get_priority_max;
    t[147] = w::__x64_sys_sched_get_priority_min;
    t[148] = w::__x64_sys_sched_rr_get_interval;
    t[149] = w::__x64_sys_mlock;
    t[150] = w::__x64_sys_munlock;
    t[151] = w::__x64_sys_mlockall;
    t[152] = w::__x64_sys_munlockall;
    t[153] = w::__x64_sys_vhangup;
    t[154] = w::__x64_sys_modify_ldt;
    t[155] = w::__x64_sys_pivot_root;
    t[157] = w::__x64_sys_prctl;
    t[158] = w::__x64_sys_arch_prctl;
    t[159] = w::__x64_sys_adjtimex;
    t[160] = w::__x64_sys_setrlimit;
    t[161] = w::__x64_sys_chroot;
    t[162] = w::__x64_sys_sync;
    t[163] = w::__x64_sys_acct;
    t[164] = w::__x64_sys_settimeofday;
    t[165] = w::__x64_sys_mount;
    t[166] = w::__x64_sys_umount2;
    t[167] = w::__x64_sys_swapon;
    t[168] = w::__x64_sys_swapoff;
    t[169] = w::__x64_sys_reboot;
    t[170] = w::__x64_sys_sethostname;
    t[171] = w::__x64_sys_setdomainname;
    t[172] = w::__x64_sys_iopl;
    t[173] = w::__x64_sys_ioperm;
    t[175] = w::__x64_sys_init_module;
    t[176] = w::__x64_sys_delete_module;
    t[179] = w::__x64_sys_quotactl;
    t[186] = w::__x64_sys_gettid;
    t[187] = w::__x64_sys_readahead;
    t[188] = w::__x64_sys_setxattr;
    t[189] = w::__x64_sys_lsetxattr;
    t[190] = w::__x64_sys_fsetxattr;
    t[191] = w::__x64_sys_getxattr;
    t[192] = w::__x64_sys_lgetxattr;
    t[193] = w::__x64_sys_fgetxattr;
    t[194] = w::__x64_sys_listxattr;
    t[195] = w::__x64_sys_llistxattr;
    t[196] = w::__x64_sys_flistxattr;
    t[197] = w::__x64_sys_removexattr;
    t[198] = w::__x64_sys_lremovexattr;
    t[199] = w::__x64_sys_fremovexattr;
    t[201] = w::__x64_sys_time;
    t[202] = w::__x64_sys_futex;
    t[203] = w::__x64_sys_sched_setaffinity;
    t[204] = w::__x64_sys_sched_getaffinity;
    t[205] = w::__x64_sys_set_thread_area;
    t[206] = w::__x64_sys_io_setup;
    t[207] = w::__x64_sys_io_destroy;
    t[208] = w::__x64_sys_io_getevents;
    t[209] = w::__x64_sys_io_submit;
    t[210] = w::__x64_sys_io_cancel;
    t[211] = w::__x64_sys_get_thread_area;
    t[213] = w::__x64_sys_epoll_create;
    t[216] = w::__x64_sys_remap_file_pages;
    t[217] = w::__x64_sys_getdents64;
    t[218] = w::__x64_sys_set_tid_address;
    t[219] = w::__x64_sys_restart_syscall;
    t[220] = w::__x64_sys_semtimedop;
    t[221] = w::__x64_sys_fadvise64;
    t[222] = w::__x64_sys_timer_create;
    t[223] = w::__x64_sys_timer_settime;
    t[224] = w::__x64_sys_timer_gettime;
    t[225] = w::__x64_sys_timer_getoverrun;
    t[226] = w::__x64_sys_timer_delete;
    t[227] = w::__x64_sys_clock_settime;
    t[228] = w::__x64_sys_clock_gettime;
    t[229] = w::__x64_sys_clock_getres;
    t[230] = w::__x64_sys_clock_nanosleep;
    t[235] = w::__x64_sys_utimes;
    t[237] = w::__x64_sys_mbind;
    t[238] = w::__x64_sys_set_mempolicy;
    t[239] = w::__x64_sys_get_mempolicy;
    t[240] = w::__x64_sys_mq_open;
    t[241] = w::__x64_sys_mq_unlink;
    t[242] = w::__x64_sys_mq_timedsend;
    t[243] = w::__x64_sys_mq_timedreceive;
    t[244] = w::__x64_sys_mq_notify;
    t[245] = w::__x64_sys_mq_getsetattr;
    t[246] = w::__x64_sys_kexec_load;
    t[253] = w::__x64_sys_inotify_init;
    t[256] = w::__x64_sys_migrate_pages;
    t[257] = w::__x64_sys_openat;
    t[258] = w::__x64_sys_mkdirat;
    t[259] = w::__x64_sys_mknodat;
    t[260] = w::__x64_sys_fchownat;
    t[261] = w::__x64_sys_futimesat;
    t[262] = w::__x64_sys_newfstatat;
    t[263] = w::__x64_sys_unlinkat;
    t[264] = w::__x64_sys_renameat;
    t[265] = w::__x64_sys_linkat;
    t[266] = w::__x64_sys_symlinkat;
    t[267] = w::__x64_sys_readlinkat;
    t[268] = w::__x64_sys_fchmodat;
    t[269] = w::__x64_sys_faccessat;
    t[270] = w::__x64_sys_pselect6;
    t[271] = w::__x64_sys_ppoll;
    t[272] = w::__x64_sys_unshare;
    t[273] = w::__x64_sys_set_robust_list;
    t[274] = w::__x64_sys_get_robust_list;
    t[275] = w::__x64_sys_splice;
    t[276] = w::__x64_sys_tee;
    t[281] = w::__x64_sys_epoll_pwait;
    t[282] = w::__x64_sys_signalfd;
    t[283] = w::__x64_sys_timerfd_create;
    t[277] = w::__x64_sys_sync_file_range;
    t[278] = w::__x64_sys_vmsplice;
    t[279] = w::__x64_sys_move_pages;
    t[280] = w::__x64_sys_utimensat;
    t[286] = w::__x64_sys_timerfd_settime;
    t[287] = w::__x64_sys_timerfd_gettime;
    t[292] = w::__x64_sys_dup3;
    t[297] = w::__x64_sys_rt_tgsigqueueinfo;
    t[316] = w::__x64_sys_renameat2;
    t[319] = w::__x64_sys_memfd_create;
    t[323] = w::__x64_sys_userfaultfd;
    t[326] = w::__x64_sys_copy_file_range;
    t[313] = w::__x64_sys_finit_module;
    t[314] = w::__x64_sys_sched_setattr;
    t[315] = w::__x64_sys_sched_getattr;
    t[318] = w::__x64_sys_getrandom;
    t[324] = w::__x64_sys_membarrier;
    t[325] = w::__x64_sys_mlock2;
    t[329] = w::__x64_sys_pkey_mprotect;
    t[330] = w::__x64_sys_pkey_alloc;
    t[331] = w::__x64_sys_pkey_free;
    t[306] = w::__x64_sys_syncfs;
    t[302] = w::__x64_sys_prlimit64;
    t[303] = w::__x64_sys_name_to_handle_at;
    t[304] = w::__x64_sys_open_by_handle_at;
    t[305] = w::__x64_sys_clock_adjtime;
    t[299] = w::__x64_sys_recvmmsg;
    t[307] = w::__x64_sys_sendmmsg;
    t[309] = w::__x64_sys_getcpu;
    t[310] = w::__x64_sys_process_vm_readv;
    t[311] = w::__x64_sys_process_vm_writev;
    t[312] = w::__x64_sys_kcmp;
    t[320] = w::__x64_sys_kexec_file_load;
    t[333] = w::__x64_sys_io_pgetevents;
    t[437] = w::__x64_sys_openat2;
    t[428] = w::__x64_sys_open_tree;
    t[429] = w::__x64_sys_move_mount;
    t[430] = w::__x64_sys_fsopen;
    t[431] = w::__x64_sys_fsconfig;
    t[432] = w::__x64_sys_fsmount;
    t[433] = w::__x64_sys_fspick;
    t[434] = w::__x64_sys_pidfd_open;
    t[438] = w::__x64_sys_pidfd_getfd;
    t[439] = w::__x64_sys_faccessat2;
    t[441] = w::__x64_sys_epoll_pwait2;
    t[442] = w::__x64_sys_mount_setattr;
    t[443] = w::__x64_sys_quotactl_fd;
    t[452] = w::__x64_sys_fchmodat2;
    t[440] = w::__x64_sys_process_madvise;
    t[447] = w::__x64_sys_memfd_secret;
    t[448] = w::__x64_sys_process_mrelease;
    t[449] = w::__x64_sys_futex_waitv;
    t[450] = w::__x64_sys_set_mempolicy_home_node;
    t[451] = w::__x64_sys_cachestat;
    t[453] = w::__x64_sys_map_shadow_stack;
    t[454] = w::__x64_sys_futex_wake;
    t[455] = w::__x64_sys_futex_wait;
    t[456] = w::__x64_sys_futex_requeue;
    t[457] = w::__x64_sys_statmount;
    t[458] = w::__x64_sys_listmount;
    t[463] = w::__x64_sys_setxattrat;
    t[464] = w::__x64_sys_getxattrat;
    t[465] = w::__x64_sys_listxattrat;
    t[466] = w::__x64_sys_removexattrat;
    t[467] = w::__x64_sys_open_tree_attr;
    t[468] = w::__x64_sys_file_getattr;
    t[469] = w::__x64_sys_file_setattr;
    t[459] = w::__x64_sys_lsm_get_self_attr;
    t[460] = w::__x64_sys_lsm_set_self_attr;
    t[461] = w::__x64_sys_lsm_list_modules;
    t[462] = w::__x64_sys_mseal;
    t[470] = w::__x64_sys_listns;
    t[471] = w::__x64_sys_rseq_slice_yield;

    // ── Signals (M25) ──
    t[13] = w::__x64_sys_rt_sigaction;
    t[14] = w::__x64_sys_rt_sigprocmask;
    t[15] = w::__x64_sys_rt_sigreturn;
    t[127] = w::__x64_sys_rt_sigpending;
    t[128] = w::__x64_sys_rt_sigtimedwait;
    t[129] = w::__x64_sys_rt_sigqueueinfo;
    t[130] = w::__x64_sys_rt_sigsuspend;
    t[131] = w::__x64_sys_sigaltstack;
    t[132] = w::__x64_sys_utime;
    t[139] = w::__x64_sys_sysfs;
    t[200] = w::__x64_sys_tkill;
    t[234] = w::__x64_sys_tgkill;

    // ── Task — clone / fork / execve (M23, M24) ──
    t[56] = w::__x64_sys_clone;
    t[57] = w::__x64_sys_fork;
    t[58] = w::__x64_sys_vfork;
    t[59] = w::__x64_sys_execve;
    t[322] = w::__x64_sys_execveat;

    // ── Exit / wait / ptrace (M26) ──
    t[60] = w::__x64_sys_exit;
    t[61] = w::__x64_sys_wait4;
    t[63] = w::__x64_sys_uname;
    t[101] = w::__x64_sys_ptrace;
    t[231] = w::__x64_sys_exit_group;
    t[247] = w::__x64_sys_waitid;
    t[308] = w::__x64_sys_setns;
    t[317] = w::__x64_sys_seccomp;
    t[435] = w::__x64_sys_clone3;
    t[436] = w::__x64_sys_close_range;

    // ── M60 — event/notification fds + io_uring ──
    t[232] = w::__x64_sys_epoll_wait;
    t[233] = w::__x64_sys_epoll_ctl;
    t[254] = w::__x64_sys_inotify_add_watch;
    t[255] = w::__x64_sys_inotify_rm_watch;
    t[284] = w::__x64_sys_eventfd;
    t[285] = w::__x64_sys_fallocate;
    t[288] = w::__x64_sys_accept4;
    t[289] = w::__x64_sys_signalfd4;
    t[290] = w::__x64_sys_eventfd2;
    t[291] = w::__x64_sys_epoll_create1;
    t[293] = w::__x64_sys_pipe2;
    t[295] = w::__x64_sys_preadv;
    t[296] = w::__x64_sys_pwritev;
    t[294] = w::__x64_sys_inotify_init1;
    t[300] = w::__x64_sys_fanotify_init;
    t[301] = w::__x64_sys_fanotify_mark;
    t[424] = w::__x64_sys_pidfd_send_signal;
    t[425] = w::__x64_sys_io_uring_setup;
    t[426] = w::__x64_sys_io_uring_enter;
    t[427] = w::__x64_sys_io_uring_register;

    // ── M63 — perf_event_open + sys_bpf ──
    t[298] = w::__x64_sys_perf_event_open;
    t[321] = w::__x64_sys_bpf;
    t[332] = w::__x64_sys_statx;
    t[334] = w::__x64_sys_rseq;
    t[335] = w::__x64_sys_uretprobe;
    t[336] = w::__x64_sys_uprobe;
    t[327] = w::__x64_sys_preadv2;
    t[328] = w::__x64_sys_pwritev2;

    // ── M64 — keyring + landlock ──
    t[250] = w::__x64_sys_keyctl;
    t[251] = w::__x64_sys_ioprio_set;
    t[252] = w::__x64_sys_ioprio_get;
    t[248] = w::__x64_sys_add_key;
    t[249] = w::__x64_sys_request_key;
    t[444] = w::__x64_sys_landlock_create_ruleset;
    t[445] = w::__x64_sys_landlock_add_rule;
    t[446] = w::__x64_sys_landlock_restrict_self;

    t
}

pub static SYS_CALL_TABLE: [SyscallFn; NR_syscalls] = build_table();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_length() {
        assert_eq!(SYS_CALL_TABLE.len(), NR_syscalls);
        assert_eq!(NR_syscalls, 472);
    }

    #[test]
    fn test_known_slots_wired() {
        // Pointer identity: implemented slots must NOT be sys_ni_syscall.
        let ni = sys_ni_syscall as usize;
        for slot in [
            0usize, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
            23, 24, 25, 26, 27, 28, 32, 33, 34, 35, 36, 37, 38, 39, 41, 42, 43, 44, 45, 46, 47, 48,
            49, 50, 51, 52, 53, 54, 55, 56, 57, 59, 60, 61, 62, 63, 72, 73, 74, 75, 76, 77, 83, 84,
            85, 87, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 104, 105, 106, 107, 108,
            109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125,
            126, 127, 128, 129, 130, 131, 135, 137, 138, 140, 141, 142, 143, 144, 145, 146, 147,
            148, 149, 150, 151, 152, 157, 158, 160, 162, 165, 170, 171, 172, 173, 175, 176, 179,
            186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199, 200, 201, 202,
            203, 204, 205, 211, 213, 217, 218, 219, 221, 222, 223, 224, 225, 226, 227, 228, 229,
            230, 231, 232, 233, 234, 247, 248, 249, 250, 251, 252, 254, 255, 257, 258, 260, 262,
            263, 268, 269, 270, 271, 272, 273, 274, 277, 281, 282, 283, 284, 285, 286, 287, 288,
            289, 290, 291, 292, 293, 294, 295, 296, 298, 299, 300, 301, 302, 306, 307, 308, 309,
            313, 314, 315, 317, 318, 321, 322, 324, 325, 327, 328, 329, 330, 331, 332, 425, 426,
            427, 436, 435, 437, 439, 441, 444, 445, 446, 449, 452, 454, 455, 456,
        ] {
            assert_ne!(
                SYS_CALL_TABLE[slot] as usize, ni,
                "slot {slot} should be wired"
            );
        }
    }

    #[test]
    fn test_unimplemented_slots_are_enosys() {
        let ni = sys_ni_syscall as usize;
        // Linux x86_64 reserved table holes stay locked to -ENOSYS.
        assert_eq!(SYS_CALL_TABLE[134] as usize, ni);
        assert_eq!(SYS_CALL_TABLE[236] as usize, ni);
    }

    #[test]
    fn syscall_m76_linux_sys_ni_parity() {
        let ni = sys_ni_syscall as usize;
        assert_eq!(SYS_CALL_TABLE[156] as usize, ni); // _sysctl is sys_ni in Linux x86_64.
    }
}
