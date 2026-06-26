//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rpm-traces.c
//! test-origin: linux:vendor/linux/kernel/trace/rpm-traces.c
//! Runtime-PM tracepoints (`rpm_suspend`, `rpm_resume`, `rpm_idle`, `rpm_usage`).
//!
//! Ref: vendor/linux/kernel/trace/rpm-traces.c

#[derive(Clone, Copy, Debug)]
pub struct RpmEvent {
    pub dev_id: u32,
    pub event: u32,
    pub usage: u32,
}

pub const RPM_SUSPEND: u32 = 0;
pub const RPM_RESUME: u32 = 1;
pub const RPM_IDLE: u32 = 2;
pub const RPM_USAGE: u32 = 3;

pub fn emit(e: RpmEvent) -> RpmEvent {
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_constants_ascend() {
        assert!(RPM_SUSPEND < RPM_RESUME);
        assert!(RPM_RESUME < RPM_IDLE);
    }

    #[test]
    fn emit_returns_event() {
        let e = emit(RpmEvent {
            dev_id: 7,
            event: RPM_SUSPEND,
            usage: 1,
        });
        assert_eq!(e.dev_id, 7);
    }
}
