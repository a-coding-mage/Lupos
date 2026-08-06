// SPDX-License-Identifier: ((GPL-2.0 WITH Linux-syscall-note) OR BSD-3-Clause)
//! linux-source: include/uapi/linux/dev_energymodel.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016099

//! YNL-generated dev-energymodel generic-netlink UAPI definitions.

use core::ffi::{c_char, c_int};

// C enum tags name the C `int` ABI type.  The distinct Rust value namespace
// retains the C-style enumerator initializers while preserving that ABI.
macro_rules! dev_energymodel_uapi_enum {
    ($name:ident) => {
        pub type $name = c_int;

        #[allow(non_snake_case)]
        const fn $name(value: c_int) -> c_int {
            value
        }
    };
}

dev_energymodel_uapi_enum!(dev_energymodel_perf_state_flags);
dev_energymodel_uapi_enum!(dev_energymodel_perf_domain_flags);

// C string-literal macros are NUL-terminated `char` arrays with static
// storage; callers use `.as_ptr()` at a C expression-context pointer-decay
// boundary.
pub static DEV_ENERGYMODEL_FAMILY_NAME: [c_char; 16] = [
    b'd' as c_char,
    b'e' as c_char,
    b'v' as c_char,
    b'-' as c_char,
    b'e' as c_char,
    b'n' as c_char,
    b'e' as c_char,
    b'r' as c_char,
    b'g' as c_char,
    b'y' as c_char,
    b'm' as c_char,
    b'o' as c_char,
    b'd' as c_char,
    b'e' as c_char,
    b'l' as c_char,
    0,
];
pub const DEV_ENERGYMODEL_FAMILY_VERSION: c_int = 1;

pub const DEV_ENERGYMODEL_PERF_STATE_FLAGS_PERF_STATE_INEFFICIENT:
    dev_energymodel_perf_state_flags = dev_energymodel_perf_state_flags(1);

pub const DEV_ENERGYMODEL_PERF_DOMAIN_FLAGS_PERF_DOMAIN_MICROWATTS:
    dev_energymodel_perf_domain_flags = dev_energymodel_perf_domain_flags(1);
pub const DEV_ENERGYMODEL_PERF_DOMAIN_FLAGS_PERF_DOMAIN_SKIP_INEFFICIENCIES:
    dev_energymodel_perf_domain_flags = dev_energymodel_perf_domain_flags(2);
pub const DEV_ENERGYMODEL_PERF_DOMAIN_FLAGS_PERF_DOMAIN_ARTIFICIAL:
    dev_energymodel_perf_domain_flags = dev_energymodel_perf_domain_flags(4);

pub const DEV_ENERGYMODEL_A_PERF_DOMAIN_PAD: c_int = 1;
pub const DEV_ENERGYMODEL_A_PERF_DOMAIN_PERF_DOMAIN_ID: c_int = 2;
pub const DEV_ENERGYMODEL_A_PERF_DOMAIN_FLAGS: c_int = 3;
pub const DEV_ENERGYMODEL_A_PERF_DOMAIN_CPUS: c_int = 4;
pub const __DEV_ENERGYMODEL_A_PERF_DOMAIN_MAX: c_int = 5;
pub const DEV_ENERGYMODEL_A_PERF_DOMAIN_MAX: c_int = __DEV_ENERGYMODEL_A_PERF_DOMAIN_MAX - 1;

pub const DEV_ENERGYMODEL_A_PERF_TABLE_PERF_DOMAIN_ID: c_int = 1;
pub const DEV_ENERGYMODEL_A_PERF_TABLE_PERF_STATE: c_int = 2;
pub const __DEV_ENERGYMODEL_A_PERF_TABLE_MAX: c_int = 3;
pub const DEV_ENERGYMODEL_A_PERF_TABLE_MAX: c_int = __DEV_ENERGYMODEL_A_PERF_TABLE_MAX - 1;

pub const DEV_ENERGYMODEL_A_PERF_STATE_PAD: c_int = 1;
pub const DEV_ENERGYMODEL_A_PERF_STATE_PERFORMANCE: c_int = 2;
pub const DEV_ENERGYMODEL_A_PERF_STATE_FREQUENCY: c_int = 3;
pub const DEV_ENERGYMODEL_A_PERF_STATE_POWER: c_int = 4;
pub const DEV_ENERGYMODEL_A_PERF_STATE_COST: c_int = 5;
pub const DEV_ENERGYMODEL_A_PERF_STATE_FLAGS: c_int = 6;
pub const __DEV_ENERGYMODEL_A_PERF_STATE_MAX: c_int = 7;
pub const DEV_ENERGYMODEL_A_PERF_STATE_MAX: c_int = __DEV_ENERGYMODEL_A_PERF_STATE_MAX - 1;

pub const DEV_ENERGYMODEL_CMD_GET_PERF_DOMAINS: c_int = 1;
pub const DEV_ENERGYMODEL_CMD_GET_PERF_TABLE: c_int = 2;
pub const DEV_ENERGYMODEL_CMD_PERF_DOMAIN_CREATED: c_int = 3;
pub const DEV_ENERGYMODEL_CMD_PERF_DOMAIN_UPDATED: c_int = 4;
pub const DEV_ENERGYMODEL_CMD_PERF_DOMAIN_DELETED: c_int = 5;
pub const __DEV_ENERGYMODEL_CMD_MAX: c_int = 6;
pub const DEV_ENERGYMODEL_CMD_MAX: c_int = __DEV_ENERGYMODEL_CMD_MAX - 1;

pub static DEV_ENERGYMODEL_MCGRP_EVENT: [c_char; 6] = [
    b'e' as c_char,
    b'v' as c_char,
    b'e' as c_char,
    b'n' as c_char,
    b't' as c_char,
    0,
];
