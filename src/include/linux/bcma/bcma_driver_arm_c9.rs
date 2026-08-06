// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/bcma/bcma_driver_arm_c9.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013499

// DMU (Device Management Unit)
//
// These are C unsuffixed hexadecimal integer literals.  Each value is
// representable by the frozen targets' signed `int`, so each is `i32` here.
// A translated expression combining one with an unsigned C operand must make
// the same C usual-arithmetic conversion explicitly at that expression.
pub const BCMA_DMU_CRU_USB2_CONTROL: i32 = 0x0164;
pub const BCMA_DMU_CRU_USB2_CONTROL_USB_PLL_NDIV_MASK: i32 = 0x0000_0ffc;
pub const BCMA_DMU_CRU_USB2_CONTROL_USB_PLL_NDIV_SHIFT: i32 = 2;
pub const BCMA_DMU_CRU_USB2_CONTROL_USB_PLL_PDIV_MASK: i32 = 0x0000_7000;
pub const BCMA_DMU_CRU_USB2_CONTROL_USB_PLL_PDIV_SHIFT: i32 = 12;
pub const BCMA_DMU_CRU_CLKSET_KEY: i32 = 0x0180;
pub const BCMA_DMU_CRU_STRAPS_CTRL: i32 = 0x02a0;
pub const BCMA_DMU_CRU_STRAPS_CTRL_USB3: i32 = 0x0000_0010;
pub const BCMA_DMU_CRU_STRAPS_CTRL_4BYTE: i32 = 0x0000_8000;
