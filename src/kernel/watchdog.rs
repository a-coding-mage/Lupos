//! linux-parity: partial
//! linux-source: vendor/linux/kernel/watchdog.c
//! test-origin: linux:vendor/linux/kernel/watchdog.c
//! Soft-lockup watchdog.
//!
//! Mirrors the core policy from `kernel/watchdog.c`: a watchdog threshold of
//! 10 seconds, a soft-lockup threshold of `2 * watchdog_thresh`, and periodic
//! tick-side checks that report a stuck current task.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::arch::x86::kernel::idt::ExceptionFrame;
use crate::kernel::sched::MAX_CPUS;

pub const WATCHDOG_THRESH_DEFAULT_SECS: u64 = 10;
pub const NUM_SAMPLE_PERIODS: u64 = 5;
pub const NSEC_PER_SEC: u64 = 1_000_000_000;
pub const SOFTLOCKUP_DELAY_REPORT: u64 = u64::MAX;

static WATCHDOG_ENABLED: [AtomicBool; MAX_CPUS] = [const { AtomicBool::new(false) }; MAX_CPUS];
static WATCHDOG_TOUCH_TS: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static WATCHDOG_REPORT_TS: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static WATCHDOG_REPORT_COUNT: [AtomicU64; MAX_CPUS] = [const { AtomicU64::new(0) }; MAX_CPUS];
static WATCHDOG_THRESH_SECS: AtomicU64 = AtomicU64::new(WATCHDOG_THRESH_DEFAULT_SECS);

struct WatchdogSerial;

impl core::fmt::Write for WatchdogSerial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        crate::linux_driver_abi::tty::serial::enqueue_bytes(s.as_bytes());
        Ok(())
    }
}

fn watchdog_serial_println(args: core::fmt::Arguments<'_>) {
    use core::fmt::Write;

    let mut serial = WatchdogSerial;
    let _ = serial.write_fmt(args);
    let _ = serial.write_str("\n");
    let _ = crate::linux_driver_abi::tty::serial::flush_budget(1024);
}

pub const fn get_softlockup_thresh(watchdog_thresh_secs: u64) -> u64 {
    watchdog_thresh_secs.saturating_mul(2)
}

pub const fn watchdog_sample_period_ns(watchdog_thresh_secs: u64) -> u64 {
    get_softlockup_thresh(watchdog_thresh_secs).saturating_mul(NSEC_PER_SEC) / NUM_SAMPLE_PERIODS
}

fn current_cpu_index() -> usize {
    #[cfg(test)]
    {
        0
    }
    #[cfg(not(test))]
    {
        crate::arch::x86::kernel::smp::current_cpu_id().min(MAX_CPUS - 1)
    }
}

fn timestamp_secs() -> u64 {
    crate::kernel::time::timekeeping::ktime_get() / NSEC_PER_SEC
}

fn frame_is_user_mode(frame: Option<&ExceptionFrame>) -> bool {
    frame.map(|frame| frame.cs & 0x3 == 0x3).unwrap_or(false)
}

fn touch_cpu(cpu: usize, now: u64) {
    if cpu >= MAX_CPUS {
        return;
    }
    WATCHDOG_TOUCH_TS[cpu].store(now, Ordering::Release);
    WATCHDOG_REPORT_TS[cpu].store(SOFTLOCKUP_DELAY_REPORT, Ordering::Release);
}

pub fn lockup_detector_init() {
    let now = timestamp_secs();
    for cpu in 0..MAX_CPUS {
        WATCHDOG_TOUCH_TS[cpu].store(now, Ordering::Release);
        WATCHDOG_REPORT_TS[cpu].store(0, Ordering::Release);
        WATCHDOG_REPORT_COUNT[cpu].store(0, Ordering::Release);
        WATCHDOG_ENABLED[cpu].store(true, Ordering::Release);
    }
}

pub fn touch_softlockup_watchdog() {
    touch_cpu(current_cpu_index(), timestamp_secs());
}

pub fn touch_softlockup_watchdog_sched() {
    touch_softlockup_watchdog();
}

pub fn softlockup_report_count(cpu: usize) -> u64 {
    WATCHDOG_REPORT_COUNT
        .get(cpu)
        .map(|slot| slot.load(Ordering::Acquire))
        .unwrap_or(0)
}

fn softlockup_duration(touch_ts: u64, report_ts: u64, now: u64, soft_thresh: u64) -> Option<u64> {
    if touch_ts == 0 {
        return None;
    }
    let duration = now.saturating_sub(touch_ts);
    if duration < soft_thresh {
        return None;
    }
    if report_ts != 0 && now.saturating_sub(report_ts) < soft_thresh {
        return None;
    }
    Some(duration)
}

fn comm_as_str(comm: &[u8; crate::kernel::task::TASK_COMM_LEN]) -> &str {
    let len = comm.iter().position(|&b| b == 0).unwrap_or(comm.len());
    core::str::from_utf8(&comm[..len]).unwrap_or("<nonutf8>")
}

fn report_softlockup(cpu: usize, duration: u64, frame: Option<&ExceptionFrame>) {
    let current = unsafe { crate::kernel::sched::get_current() };
    let (pid, tgid, comm) = if current.is_null() {
        (-1, -1, "unknown")
    } else {
        unsafe {
            (
                (*current).pid,
                (*current).tgid,
                comm_as_str(&(*current).comm),
            )
        }
    };

    watchdog_serial_println(format_args!(
        "BUG: soft lockup - CPU#{} stuck for {}s! [{}:{}] tgid={}",
        cpu, duration, comm, pid, tgid
    ));
    if let Some(frame) = frame {
        watchdog_serial_println(format_args!(
            "soft lockup: rip={:#x} rsp={:#x} rflags={:#x} rbp={:#x}",
            frame.rip, frame.user_rsp, frame.rflags, frame.rbp
        ));
        let mut rbp = frame.rbp as *const u64;
        for depth in 0..12 {
            let rbp_addr = rbp as u64;
            if rbp_addr < crate::arch::x86::mm::paging::PAGE_OFFSET || rbp_addr & 0x7 != 0 {
                break;
            }
            let next = unsafe { core::ptr::read_unaligned(rbp) };
            let ret = unsafe { core::ptr::read_unaligned(rbp.add(1)) };
            watchdog_serial_println(format_args!(
                "soft lockup: bt{} rbp={:#x} ret={:#x}",
                depth, rbp_addr, ret
            ));
            if next <= rbp_addr || next & 0x7 != 0 {
                break;
            }
            rbp = next as *const u64;
        }
        if frame.user_rsp >= 0x1000 && frame.user_rsp & 0x7 == 0 {
            let stack = frame.user_rsp as *const u64;
            for index in 0..96 {
                let word = unsafe { core::ptr::read_unaligned(stack.add(index)) };
                if (0x0020_0000..0x0090_0000).contains(&word) {
                    watchdog_serial_println(format_args!(
                        "soft lockup: stack[{}] ret={:#x}",
                        index, word
                    ));
                }
            }
        }
    } else {
        watchdog_serial_println(format_args!("soft lockup: interrupt frame unavailable"));
    }

    #[cfg(all(feature = "test-softlockup-watchdog", feature = "qemu-test"))]
    {
        crate::linux_driver_abi::platform::qemu::exit_success();
    }
}

fn watchdog_tick_at(cpu: usize, now: u64, frame: Option<&ExceptionFrame>, emit: bool) -> bool {
    if cpu >= MAX_CPUS || !WATCHDOG_ENABLED[cpu].load(Ordering::Acquire) {
        return false;
    }

    let touch_ts = WATCHDOG_TOUCH_TS[cpu].load(Ordering::Acquire);
    if touch_ts == 0 {
        WATCHDOG_TOUCH_TS[cpu].store(now, Ordering::Release);
        return false;
    }

    let report_ts = WATCHDOG_REPORT_TS[cpu].load(Ordering::Acquire);
    if report_ts == SOFTLOCKUP_DELAY_REPORT {
        WATCHDOG_REPORT_TS[cpu].store(now, Ordering::Release);
        return false;
    }

    let soft_thresh = get_softlockup_thresh(WATCHDOG_THRESH_SECS.load(Ordering::Acquire));
    let Some(duration) = softlockup_duration(touch_ts, report_ts, now, soft_thresh) else {
        return false;
    };

    if WATCHDOG_REPORT_TS[cpu]
        .compare_exchange(report_ts, now, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    WATCHDOG_REPORT_COUNT[cpu].fetch_add(1, Ordering::AcqRel);
    if emit {
        report_softlockup(cpu, duration, frame);
    }
    true
}

pub fn watchdog_tick(cpu: usize, frame: Option<&ExceptionFrame>) {
    let cpu = cpu.min(MAX_CPUS - 1);
    let now = timestamp_secs();
    if frame_is_user_mode(frame) {
        touch_cpu(cpu, now);
        return;
    }
    let _ = watchdog_tick_at(cpu, now, frame, true);
}

#[cfg(feature = "test-softlockup-watchdog")]
pub fn run_softlockup_watchdog_test() -> ! {
    unsafe extern "C" fn stall(_arg: *mut core::ffi::c_void) -> ! {
        crate::kernel::locking::local_irq_enable();
        watchdog_serial_println(format_args!("soft-lockup-watchdog: stall thread running"));
        loop {
            core::hint::spin_loop();
        }
    }

    WATCHDOG_THRESH_SECS.store(1, Ordering::Release);
    lockup_detector_init();
    let task = unsafe {
        crate::kernel::sched::kthread_create(
            stall,
            core::ptr::null_mut(),
            b"wdogstall\0\0\0\0\0\0\0",
        )
    };
    if task.is_null() {
        panic!("soft-lockup-watchdog: failed to create stall kthread");
    }
    unsafe {
        crate::kernel::sched::enqueue_task(task);
        crate::kernel::sched::schedule();
    }
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn reset() {
        WATCHDOG_THRESH_SECS.store(WATCHDOG_THRESH_DEFAULT_SECS, Ordering::Release);
        for cpu in 0..MAX_CPUS {
            WATCHDOG_ENABLED[cpu].store(false, Ordering::Release);
            WATCHDOG_TOUCH_TS[cpu].store(0, Ordering::Release);
            WATCHDOG_REPORT_TS[cpu].store(0, Ordering::Release);
            WATCHDOG_REPORT_COUNT[cpu].store(0, Ordering::Release);
        }
    }

    #[test]
    fn watchdog_constants_match_linux_source() {
        let _guard = TEST_LOCK.lock();
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/watchdog.c"
        ));
        assert!(source.contains("int __read_mostly watchdog_thresh = 10;"));
        assert!(source.contains("#define NUM_SAMPLE_PERIODS\t5"));
        assert!(source.contains("watchdog_thresh * 2"));
        assert_eq!(WATCHDOG_THRESH_DEFAULT_SECS, 10);
        assert_eq!(get_softlockup_thresh(WATCHDOG_THRESH_DEFAULT_SECS), 20);
        assert_eq!(
            watchdog_sample_period_ns(WATCHDOG_THRESH_DEFAULT_SECS),
            4_000_000_000
        );
    }

    #[test]
    fn watchdog_reports_after_soft_threshold() {
        let _guard = TEST_LOCK.lock();
        reset();
        WATCHDOG_THRESH_SECS.store(1, Ordering::Release);
        WATCHDOG_ENABLED[0].store(true, Ordering::Release);
        WATCHDOG_TOUCH_TS[0].store(10, Ordering::Release);

        assert!(!watchdog_tick_at(0, 11, None, false));
        assert!(watchdog_tick_at(0, 12, None, false));
        assert_eq!(softlockup_report_count(0), 1);
    }

    #[test]
    fn watchdog_delay_report_sentinel_suppresses_one_period() {
        let _guard = TEST_LOCK.lock();
        reset();
        WATCHDOG_THRESH_SECS.store(1, Ordering::Release);
        WATCHDOG_ENABLED[0].store(true, Ordering::Release);
        WATCHDOG_TOUCH_TS[0].store(10, Ordering::Release);
        WATCHDOG_REPORT_TS[0].store(SOFTLOCKUP_DELAY_REPORT, Ordering::Release);

        assert!(!watchdog_tick_at(0, 20, None, false));
        assert_eq!(WATCHDOG_REPORT_TS[0].load(Ordering::Acquire), 20);
        assert_eq!(softlockup_report_count(0), 0);
    }

    #[test]
    fn watchdog_touch_suppresses_report() {
        let _guard = TEST_LOCK.lock();
        reset();
        WATCHDOG_THRESH_SECS.store(1, Ordering::Release);
        WATCHDOG_ENABLED[0].store(true, Ordering::Release);
        WATCHDOG_TOUCH_TS[0].store(10, Ordering::Release);

        touch_cpu(0, 19);

        assert!(!watchdog_tick_at(0, 20, None, false));
        assert_eq!(softlockup_report_count(0), 0);
    }

    #[test]
    fn user_mode_frames_are_not_softlockup_reports() {
        let _guard = TEST_LOCK.lock();
        let mut frame = unsafe { core::mem::zeroed::<ExceptionFrame>() };
        frame.cs = 0x33;
        assert!(frame_is_user_mode(Some(&frame)));
    }

    #[test]
    fn watchdog_reports_once_per_soft_threshold() {
        let _guard = TEST_LOCK.lock();
        reset();
        WATCHDOG_THRESH_SECS.store(1, Ordering::Release);
        WATCHDOG_ENABLED[0].store(true, Ordering::Release);
        WATCHDOG_TOUCH_TS[0].store(10, Ordering::Release);

        assert!(watchdog_tick_at(0, 12, None, false));
        assert!(!watchdog_tick_at(0, 13, None, false));
        assert!(watchdog_tick_at(0, 14, None, false));
        assert_eq!(softlockup_report_count(0), 2);
    }

    #[test]
    fn watchdog_serial_report_uses_nonblocking_serial_queue() {
        let _guard = TEST_LOCK.lock();
        crate::linux_driver_abi::tty::serial::clear_capture_for_tests();

        watchdog_serial_println(format_args!("watchdog queue report"));

        assert_eq!(
            crate::linux_driver_abi::tty::serial::captured_bytes_for_tests(),
            b"watchdog queue report\r\n"
        );
    }
}
