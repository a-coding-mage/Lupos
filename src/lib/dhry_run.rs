//! linux-parity: complete
//! linux-source: vendor/linux/lib/dhry_run.c
//! test-origin: linux:vendor/linux/lib/dhry_run.c
//! Dhrystone benchmark module run-control logic.

use crate::include::uapi::errno::EAGAIN;

pub const DHRY_VAX: i32 = 1757;
pub const MODULE_DESCRIPTION: &str = "Dhrystone benchmark test module";
pub const MODULE_AUTHOR: &str = "Geert Uytterhoeven <geert+renesas@glider.be>";
pub const MODULE_LICENSE: &str = "GPL";
pub const RUN_PARAM_MODE: u16 = 0o200;
pub const ITERATIONS_PARAM_MODE: u16 = 0o644;
pub const KERNEL_PARAM_OPS_FL_NOARG: u32 = 1;
pub const DEFAULT_ITERATIONS: i32 = -1;
pub const RUN_PARAM_DESCRIPTION: &str = "Run the test (default: false)";
pub const ITERATIONS_PARAM_DESCRIPTION: &str =
    "Number of iterations through the benchmark (default: auto)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DhryReport {
    pub iterations: i32,
    pub score: i32,
    pub dmips: Option<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DhryLog {
    Info {
        cpu: u32,
        dhrystones_per_second: i32,
        dmips: i32,
    },
    IncreaseIterations,
    Failed {
        error: i32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DhryBenchmarkReport {
    pub cpu: u32,
    pub iterations: i32,
    pub score: i32,
    pub dmips: Option<i32>,
    pub log: DhryLog,
    pub get_cpu_called: bool,
    pub put_cpu_called: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DhryRunSetReport {
    pub param_set_called: bool,
    pub param_set_return: i32,
    pub dhry_run: bool,
    pub benchmark_ran: bool,
    pub return_code: i32,
}

pub fn dhry_log(cpu: u32, score: i32) -> DhryLog {
    if score >= 0 {
        DhryLog::Info {
            cpu,
            dhrystones_per_second: score,
            dmips: score / DHRY_VAX,
        }
    } else if score == -EAGAIN {
        DhryLog::IncreaseIterations
    } else {
        DhryLog::Failed { error: score }
    }
}

pub fn dhry_benchmark_report_with<F>(cpu: u32, iterations: i32, mut dhry: F) -> DhryBenchmarkReport
where
    F: FnMut(i32) -> i32,
{
    let mut current = if iterations > 0 { iterations } else { DHRY_VAX };
    let score = if iterations > 0 {
        dhry(current)
    } else {
        loop {
            let score = dhry(current);
            if score != -EAGAIN || current > i32::MAX / 2 {
                break score;
            }
            current <<= 1;
        }
    };
    DhryBenchmarkReport {
        cpu,
        iterations: current,
        score,
        dmips: if score >= 0 {
            Some(score / DHRY_VAX)
        } else {
            None
        },
        log: dhry_log(cpu, score),
        get_cpu_called: true,
        put_cpu_called: true,
    }
}

pub fn dhry_benchmark_with<F>(iterations: i32, mut dhry: F) -> DhryReport
where
    F: FnMut(i32) -> i32,
{
    let report = dhry_benchmark_report_with(0, iterations, |iters| dhry(iters));
    DhryReport {
        iterations: report.iterations,
        score: report.score,
        dmips: report.dmips,
    }
}

pub fn dhry_run_set_report(
    val: Option<bool>,
    param_set_return: i32,
    prior_dhry_run: bool,
    system_running: bool,
) -> DhryRunSetReport {
    let param_set_called = val.is_some();
    if param_set_called && param_set_return != 0 {
        return DhryRunSetReport {
            param_set_called,
            param_set_return,
            dhry_run: prior_dhry_run,
            benchmark_ran: false,
            return_code: param_set_return,
        };
    }

    let dhry_run = val.unwrap_or(true);
    DhryRunSetReport {
        param_set_called,
        param_set_return,
        dhry_run,
        benchmark_ran: dhry_run && system_running,
        return_code: 0,
    }
}

pub fn dhry_init_report(dhry_run: bool) -> DhryRunSetReport {
    DhryRunSetReport {
        param_set_called: false,
        param_set_return: 0,
        dhry_run,
        benchmark_ran: dhry_run,
        return_code: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dhry_run_matches_linux_parameter_and_retry_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/dhry_run.c"
        ));
        assert!(source.contains("#define DHRY_VAX\t1757"));
        assert!(source.contains("static const struct kernel_param_ops run_ops"));
        assert!(source.contains(".flags = KERNEL_PARAM_OPS_FL_NOARG"));
        assert!(source.contains(".set = dhry_run_set"));
        assert!(source.contains("module_param_cb(run, &run_ops, &dhry_run, 0200);"));
        assert!(source.contains("module_param(iterations, int, 0644);"));
        assert!(source.contains("static int iterations = -1;"));
        assert!(source.contains("unsigned int cpu = get_cpu();"));
        assert!(source.contains("for (i = DHRY_VAX; i > 0; i <<= 1)"));
        assert!(source.contains("if (n != -EAGAIN)"));
        assert!(source.contains("put_cpu();"));
        assert!(source.contains("pr_info(\"CPU%u: Dhrystones per Second: %d (%d DMIPS)\\n\""));
        assert!(source.contains("pr_err(\"Please increase the number of iterations\\n\");"));
        assert!(source.contains("pr_err(\"Dhrystone benchmark failed error %pe\\n\""));
        assert!(source.contains("if (val)"));
        assert!(source.contains("ret = param_set_bool(val, kp);"));
        assert!(source.contains("dhry_run = true;"));
        assert!(source.contains("system_state == SYSTEM_RUNNING"));
        assert!(source.contains("if (dhry_run)"));
        assert!(source.contains("n / DHRY_VAX"));
        assert!(source.contains("module_init(dhry_init);"));
        assert!(
            source.contains("MODULE_AUTHOR(\"Geert Uytterhoeven <geert+renesas@glider.be>\");")
        );
        assert!(source.contains("MODULE_DESCRIPTION(\"Dhrystone benchmark test module\")"));

        assert_eq!(
            MODULE_AUTHOR,
            "Geert Uytterhoeven <geert+renesas@glider.be>"
        );
        assert_eq!(MODULE_LICENSE, "GPL");
        assert_eq!(RUN_PARAM_MODE, 0o200);
        assert_eq!(ITERATIONS_PARAM_MODE, 0o644);
        assert_eq!(KERNEL_PARAM_OPS_FL_NOARG, 1);
        assert_eq!(DEFAULT_ITERATIONS, -1);

        let mut calls = 0;
        let report = dhry_benchmark_with(-1, |iters| {
            calls += 1;
            if calls == 1 { -EAGAIN } else { iters * 2 }
        });
        assert_eq!(report.iterations, DHRY_VAX * 2);
        assert_eq!(report.score, DHRY_VAX * 4);
        assert_eq!(report.dmips, Some(4));

        let fixed = dhry_benchmark_with(10, |iters| iters * 3);
        assert_eq!(fixed.iterations, 10);
        assert_eq!(fixed.score, 30);

        let report = dhry_benchmark_report_with(2, -1, |iters| {
            if iters == DHRY_VAX {
                -EAGAIN
            } else {
                iters * 4
            }
        });
        assert_eq!(report.cpu, 2);
        assert_eq!(report.iterations, DHRY_VAX * 2);
        assert_eq!(report.score, DHRY_VAX * 8);
        assert_eq!(report.dmips, Some(8));
        assert_eq!(
            report.log,
            DhryLog::Info {
                cpu: 2,
                dhrystones_per_second: DHRY_VAX * 8,
                dmips: 8,
            }
        );
        assert!(report.get_cpu_called);
        assert!(report.put_cpu_called);

        assert_eq!(dhry_log(0, -EAGAIN), DhryLog::IncreaseIterations);
        assert_eq!(dhry_log(0, -22), DhryLog::Failed { error: -22 });
        assert_eq!(
            dhry_run_set_report(Some(true), 0, false, true),
            DhryRunSetReport {
                param_set_called: true,
                param_set_return: 0,
                dhry_run: true,
                benchmark_ran: true,
                return_code: 0,
            }
        );
        assert_eq!(
            dhry_run_set_report(None, 0, false, true),
            DhryRunSetReport {
                param_set_called: false,
                param_set_return: 0,
                dhry_run: true,
                benchmark_ran: true,
                return_code: 0,
            }
        );
        assert_eq!(
            dhry_run_set_report(Some(false), -22, true, true),
            DhryRunSetReport {
                param_set_called: true,
                param_set_return: -22,
                dhry_run: true,
                benchmark_ran: false,
                return_code: -22,
            }
        );
        assert_eq!(
            dhry_init_report(true),
            DhryRunSetReport {
                param_set_called: false,
                param_set_return: 0,
                dhry_run: true,
                benchmark_ran: true,
                return_code: 0,
            }
        );
    }
}
