//! linux-parity: partial
//! linux-source: vendor/linux/kernel/trace
//! test-origin: linux:vendor/linux/kernel/trace
//! Tracing subsystem (M62) — tracepoints, ring buffer, ftrace, kprobes.
//!
//! Sub-modules:
//! - `tracepoint` — `Tracepoint` static instances + register/unregister.
//! - `ring_buffer` — `TraceRingBuffer` storing fixed-size `TraceEvent`s.
//! - `ftrace` — function tracer with global probe attach point.
//! - `kprobe` — software int3-based instruction probes (text patching deferred).

pub mod ftrace;
pub mod kprobe;
pub mod ring_buffer;
pub mod tracepoint;

// M61-M62 expanded coverage.
pub mod blktrace;
pub mod bpf_trace;
pub mod error_report_traces;
pub mod fgraph;
pub mod fprobe;
pub mod pid_list;
pub mod power_traces;
pub mod rethook;
pub mod ring_buffer_benchmark;
pub mod rpm_traces;
pub mod rv;
pub mod simple_ring_buffer;
pub mod trace;
pub mod trace_benchmark;
pub mod trace_boot;
pub mod trace_branch;
pub mod trace_btf;
pub mod trace_clock;
pub mod trace_dynevent;
pub mod trace_eprobe;
pub mod trace_event_perf;
pub mod trace_events;
pub mod trace_events_filter;
pub mod trace_events_hist;
pub mod trace_events_inject;
pub mod trace_events_synth;
pub mod trace_events_trigger;
pub mod trace_events_user;
pub mod trace_export;
pub mod trace_fprobe;
pub mod trace_functions;
pub mod trace_functions_graph;
pub mod trace_hwlat;
pub mod trace_irqsoff;
pub mod trace_kdb;
pub mod trace_kprobe;
pub mod trace_kprobe_selftest;
pub mod trace_mmiotrace;
pub mod trace_nop;
pub mod trace_osnoise;
pub mod trace_output;
pub mod trace_pid;
pub mod trace_preemptirq;
pub mod trace_printk;
pub mod trace_probe;
pub mod trace_recursion_record;
pub mod trace_remote;
pub mod trace_sched_switch;
pub mod trace_sched_wakeup;
pub mod trace_selftest_dynamic;
pub mod trace_seq;
pub mod trace_snapshot;
pub mod trace_stack;
pub mod trace_stat;
pub mod trace_syscalls;
pub mod trace_uprobe;
pub mod tracing_map;
pub mod undefsyms_base;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceCompileUnit {
    pub linux_source: &'static str,
    pub headers: &'static [&'static str],
    pub create_trace_points: bool,
    pub checker_gated: bool,
    pub exported_tracepoints: &'static [&'static str],
    pub module_description: Option<&'static str>,
}

#[cfg(test)]
pub(crate) fn assert_trace_compile_unit(source: &str, unit: TraceCompileUnit) {
    for header in unit.headers {
        assert!(
            source.contains(header),
            "{} missing {}",
            unit.linux_source,
            header
        );
    }
    if unit.create_trace_points {
        assert!(
            source.contains("#define CREATE_TRACE_POINTS"),
            "{} missing CREATE_TRACE_POINTS",
            unit.linux_source
        );
    }
    if unit.checker_gated {
        assert!(
            source.contains("#ifndef __CHECKER__"),
            "{} missing __CHECKER__ guard",
            unit.linux_source
        );
    }
    for exported in unit.exported_tracepoints {
        assert!(
            source.contains(exported),
            "{} missing {}",
            unit.linux_source,
            exported
        );
    }
    if let Some(description) = unit.module_description {
        assert!(
            source.contains(description),
            "{} missing {}",
            unit.linux_source,
            description
        );
    }
}

pub fn init() {
    // Nothing to do in M62 — all rings are static-initialised.
}
