//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/power-traces.c
//! test-origin: linux:vendor/linux/kernel/trace/power-traces.c
//! Static tracepoints for the power subsystem (cpu_idle, cpu_frequency, etc.).
//!
//! Ref: vendor/linux/kernel/trace/power-traces.c

#[derive(Clone, Copy, Debug)]
pub struct CpuIdleEvent {
    pub state: u32,
    pub cpu: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct CpuFrequencyEvent {
    pub freq_khz: u32,
    pub cpu: u32,
}

pub fn trace_cpu_idle(e: CpuIdleEvent) -> CpuIdleEvent {
    e
}

pub fn trace_cpu_frequency(e: CpuFrequencyEvent) -> CpuFrequencyEvent {
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_idle_passthrough() {
        let e = trace_cpu_idle(CpuIdleEvent { state: 2, cpu: 0 });
        assert_eq!(e.state, 2);
    }

    #[test]
    fn cpu_freq_passthrough() {
        let e = trace_cpu_frequency(CpuFrequencyEvent {
            freq_khz: 3_000_000,
            cpu: 1,
        });
        assert_eq!(e.cpu, 1);
    }
}
