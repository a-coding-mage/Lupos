//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/numa.c
//! test-origin: linux:vendor/linux/arch/x86/mm/numa.c
//! x86 NUMA fallback policy.
//!
//! Mirrors the early NUMA setup and exported single-node fallbacks from
//! `vendor/linux/arch/x86/mm/numa.c`. Lupos currently runs as node 0 only.

use crate::include::uapi::errno::EINVAL;
use crate::kernel::sched::MAX_CPUS;

pub const NUMA_NO_NODE: i32 = -1;
pub const MAX_NUMNODES: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumaCommandLine {
    Default,
    Off,
    Fake(u8),
}

pub fn numa_setup(arg: &str) -> Result<NumaCommandLine, i32> {
    match arg.as_bytes() {
        b"" => Ok(NumaCommandLine::Default),
        b"off" | b"noacpi" => Ok(NumaCommandLine::Off),
        [b'f', b'a', b'k', b'e', b'=', digit] if *digit >= b'1' && *digit <= b'9' => {
            Ok(NumaCommandLine::Fake(*digit - b'0'))
        }
        _ => Err(EINVAL),
    }
}

pub const fn numa_enabled(cmdline: NumaCommandLine) -> bool {
    matches!(cmdline, NumaCommandLine::Fake(n) if n > 1)
}

pub const fn cpu_to_node(cpu: usize) -> Result<usize, i32> {
    if cpu >= MAX_CPUS { Err(EINVAL) } else { Ok(0) }
}

pub const fn cpumask_of_node(node: usize) -> Result<u64, i32> {
    if node != 0 {
        return Err(EINVAL);
    }
    if MAX_CPUS >= 64 {
        Ok(u64::MAX)
    } else {
        Ok((1u64 << MAX_CPUS) - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numa_setup_parses_off_and_fake() {
        assert_eq!(numa_setup("off"), Ok(NumaCommandLine::Off));
        assert_eq!(numa_setup("fake=2"), Ok(NumaCommandLine::Fake(2)));
    }

    #[test]
    fn fallback_maps_all_valid_cpus_to_node_zero() {
        assert_eq!(cpu_to_node(0), Ok(0));
        assert!(cpumask_of_node(0).unwrap() & 1 != 0);
        assert_eq!(cpumask_of_node(1), Err(EINVAL));
    }
}
