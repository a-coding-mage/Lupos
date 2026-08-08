// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/pinctrl/pinctrl-state.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014648

//! Standard pin control state names.
//!
//! These constants preserve the exact string-literal values of the Linux
//! `PINCTRL_STATE_*` preprocessor macros.  The header contains no objects,
//! types, or callable declarations; its only operative content is these
//! compile-time names.

pub const PINCTRL_STATE_DEFAULT: &str = "default";
pub const PINCTRL_STATE_INIT: &str = "init";
pub const PINCTRL_STATE_IDLE: &str = "idle";
pub const PINCTRL_STATE_SLEEP: &str = "sleep";
