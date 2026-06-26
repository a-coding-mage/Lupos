//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/shstk.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/shstk.c
//! x86 user shadow-stack helpers.
//!
//! Implements a subset of shstk.c: the `arch_prctl(ARCH_SHSTK_*)` surface and
//! shadow-stack token/mapping helpers. Remaining work vs Linux for `complete`:
//! the portions of shstk.c not yet ported — full CET enable/disable across
//! clone/exec, `map_shadow_stack(2)` allocation, and signal-frame shadow-stack
//! save/restore.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/shstk.c

#![allow(dead_code)]

use crate::include::uapi::errno::{EFAULT, EINVAL, ENOSYS, EOPNOTSUPP};

pub const ARCH_SHSTK_ENABLE: i32 = 0x5001;
pub const ARCH_SHSTK_DISABLE: i32 = 0x5002;
pub const ARCH_SHSTK_LOCK: i32 = 0x5003;
pub const ARCH_SHSTK_UNLOCK: i32 = 0x5004;
pub const ARCH_SHSTK_STATUS: i32 = 0x5005;
pub const ARCH_SHSTK_SHSTK: u64 = 1 << 0;
pub const ARCH_SHSTK_WRSS: u64 = 1 << 1;

pub const SHADOW_STACK_SET_TOKEN: u32 = 1 << 0;
pub const SHADOW_STACK_SET_MARKER: u32 = 1 << 1;
pub const SS_FRAME_SIZE: u64 = 8;
pub const PAGE_SIZE: u64 = 4096;
pub const SHSTK_DATA_BIT: u64 = 1 << 63;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShadowStackState {
    pub features: u64,
    pub locked: u64,
    pub base: u64,
    pub size: u64,
    pub ssp: u64,
}

impl ShadowStackState {
    pub const fn is_enabled(self) -> bool {
        self.features & ARCH_SHSTK_SHSTK != 0
    }
}

pub const fn valid_feature_mask(features: u64) -> bool {
    features & !(ARCH_SHSTK_SHSTK | ARCH_SHSTK_WRSS) == 0
}

pub const fn adjust_shstk_size(size: u64, rlimit_stack: u64) -> u64 {
    let raw = if size == 0 {
        if rlimit_stack < (1u64 << 32) {
            rlimit_stack
        } else {
            1u64 << 32
        }
    } else {
        size
    };
    (raw + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub const fn create_rstor_token(ssp: u64) -> Result<(u64, u64), i32> {
    if ssp & 7 != 0 || ssp < SS_FRAME_SIZE {
        return Err(EINVAL);
    }
    Ok((ssp - SS_FRAME_SIZE, ssp | 1))
}

pub const fn put_shstk_data(data: u64) -> Result<u64, i32> {
    if data & SHSTK_DATA_BIT != 0 {
        return Err(EINVAL);
    }
    Ok(data | SHSTK_DATA_BIT)
}

pub const fn get_shstk_data(encoded: u64) -> Result<u64, i32> {
    if encoded & SHSTK_DATA_BIT == 0 {
        return Err(EFAULT);
    }
    Ok(encoded & !SHSTK_DATA_BIT)
}

pub fn shstk_prctl(
    state: &mut ShadowStackState,
    option: i32,
    features: u64,
    checkpoint_restore: bool,
) -> Result<u64, i32> {
    if option == ARCH_SHSTK_STATUS {
        return Ok(state.features);
    }
    if !valid_feature_mask(features) {
        return Err(EINVAL);
    }
    match option {
        ARCH_SHSTK_LOCK => {
            state.locked |= features;
            Ok(0)
        }
        ARCH_SHSTK_UNLOCK if checkpoint_restore => {
            state.locked &= !features;
            Ok(0)
        }
        ARCH_SHSTK_UNLOCK => Err(EOPNOTSUPP),
        ARCH_SHSTK_DISABLE => {
            if state.locked & features != 0 {
                return Err(EOPNOTSUPP);
            }
            if features & ARCH_SHSTK_WRSS != 0 {
                state.features &= !ARCH_SHSTK_WRSS;
            }
            if features & ARCH_SHSTK_SHSTK != 0 {
                state.features &= !(ARCH_SHSTK_SHSTK | ARCH_SHSTK_WRSS);
            }
            Ok(0)
        }
        ARCH_SHSTK_ENABLE => {
            if state.locked & features != 0 {
                return Err(EOPNOTSUPP);
            }
            if features & ARCH_SHSTK_WRSS != 0 && !state.is_enabled() {
                return Err(EINVAL);
            }
            state.features |= features;
            Ok(0)
        }
        _ => Err(EINVAL),
    }
}

pub fn sys_map_shadow_stack(_addr: u64, size: u64, flags: u32) -> i64 {
    if size == 0 || flags & !(SHADOW_STACK_SET_TOKEN | SHADOW_STACK_SET_MARKER) != 0 {
        return -(EINVAL as i64);
    }
    -(ENOSYS as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_token_is_aligned_and_marks_64bit_mode() {
        assert_eq!(create_rstor_token(0x1008), Ok((0x1000, 0x1009)));
        assert_eq!(create_rstor_token(0x1003), Err(EINVAL));
    }

    #[test]
    fn prctl_enforces_locks_and_wrss_dependency() {
        let mut state = ShadowStackState::default();
        assert_eq!(
            shstk_prctl(&mut state, ARCH_SHSTK_ENABLE, ARCH_SHSTK_WRSS, false),
            Err(EINVAL)
        );
        assert_eq!(
            shstk_prctl(&mut state, ARCH_SHSTK_ENABLE, ARCH_SHSTK_SHSTK, false),
            Ok(0)
        );
        assert_eq!(
            shstk_prctl(&mut state, ARCH_SHSTK_ENABLE, ARCH_SHSTK_WRSS, false),
            Ok(0)
        );
        assert_eq!(
            shstk_prctl(&mut state, ARCH_SHSTK_LOCK, ARCH_SHSTK_SHSTK, false),
            Ok(0)
        );
        assert_eq!(
            shstk_prctl(&mut state, ARCH_SHSTK_DISABLE, ARCH_SHSTK_SHSTK, false),
            Err(EOPNOTSUPP)
        );
    }
}
