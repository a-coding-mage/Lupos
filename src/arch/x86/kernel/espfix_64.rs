//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/espfix_64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/espfix_64.c
//! x86-64 ESPFIX ministack address calculation.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/espfix_64.c

use crate::arch::x86::mm::paging::{PAGE_SHIFT, PAGE_SIZE};

pub const P4D_SHIFT: u32 = 39;
pub const ESPFIX_PGD_ENTRY: u64 = u64::MAX - 1;
pub const ESPFIX_BASE_ADDR: u64 = ESPFIX_PGD_ENTRY << P4D_SHIFT;

pub const ESPFIX_STACK_SIZE: u64 = 8 * 8;
pub const ESPFIX_STACKS_PER_PAGE: u64 = PAGE_SIZE / ESPFIX_STACK_SIZE;
pub const ESPFIX_PAGE_SPACE: u64 = 1u64 << (P4D_SHIFT - PAGE_SHIFT - 16);
pub const ESPFIX_MAX_CPUS: u64 = ESPFIX_STACKS_PER_PAGE * ESPFIX_PAGE_SPACE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EspfixRandom {
    pub page_random: u64,
    pub slot_random: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EspfixCpuState {
    pub stack: u64,
    pub waddr: u64,
    pub shared_page: u64,
}

pub const fn init_espfix_random(rand: u64) -> EspfixRandom {
    EspfixRandom {
        slot_random: rand % ESPFIX_STACKS_PER_PAGE,
        page_random: (rand / ESPFIX_STACKS_PER_PAGE) & (ESPFIX_PAGE_SPACE - 1),
    }
}

pub const fn espfix_base_addr(cpu: u64, random: EspfixRandom) -> u64 {
    let page = (cpu / ESPFIX_STACKS_PER_PAGE) ^ random.page_random;
    let slot = (cpu + random.slot_random) % ESPFIX_STACKS_PER_PAGE;
    let addr = (page << PAGE_SHIFT) + slot * ESPFIX_STACK_SIZE;
    let alias = (addr & 0xffff) | ((addr & !0xffff) << 16);
    ESPFIX_BASE_ADDR + alias
}

pub const fn init_espfix_ap(
    cpu: u64,
    stack_page: u64,
    random: EspfixRandom,
    fred_enabled: bool,
) -> Option<EspfixCpuState> {
    if fred_enabled {
        return None;
    }
    let stack = espfix_base_addr(cpu, random);
    Some(EspfixCpuState {
        stack,
        waddr: stack_page + (stack & (PAGE_SIZE - 1)),
        shared_page: cpu / ESPFIX_STACKS_PER_PAGE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espfix_constants_match_linux_layout() {
        assert_eq!(ESPFIX_STACK_SIZE, 64);
        assert_eq!(ESPFIX_STACKS_PER_PAGE, 64);
        assert_eq!(ESPFIX_BASE_ADDR, 0xffff_ff00_0000_0000);
    }

    #[test]
    fn random_split_uses_slot_then_page_space_mask() {
        let random = init_espfix_random(65);
        assert_eq!(
            random,
            EspfixRandom {
                slot_random: 1,
                page_random: 1
            }
        );
    }

    #[test]
    fn base_addr_aliases_low_16_bits_every_64k() {
        let random = EspfixRandom {
            page_random: 0,
            slot_random: 0,
        };
        assert_eq!(espfix_base_addr(0, random), ESPFIX_BASE_ADDR);
        assert_eq!(espfix_base_addr(1, random), ESPFIX_BASE_ADDR + 64);
        assert_eq!(espfix_base_addr(64, random), ESPFIX_BASE_ADDR + 0x1000);
        assert_eq!(espfix_base_addr(1024, random), ESPFIX_BASE_ADDR + (1 << 32));
    }

    #[test]
    fn init_ap_skips_fred_and_computes_write_address() {
        let random = EspfixRandom {
            page_random: 0,
            slot_random: 1,
        };
        assert_eq!(init_espfix_ap(0, 0x1000_0000, random, true), None);
        let state = init_espfix_ap(0, 0x1000_0000, random, false).unwrap();
        assert_eq!(state.stack, ESPFIX_BASE_ADDR + 64);
        assert_eq!(state.waddr, 0x1000_0040);
    }
}
