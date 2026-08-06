// SPDX-License-Identifier: GPL-1.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/serial_reg.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016378

/*
 * Copyright (C) 1992, 1994 by Theodore Ts'o.
 *
 * UART port register offsets and bit definitions for 8250-compatible UARTs.
 */

pub const UART_RX: i32 = 0;
pub const UART_TX: i32 = 0;
pub const UART_IER: i32 = 1;
pub const UART_IER_MSI: i32 = 0x08;
pub const UART_IER_RLSI: i32 = 0x04;
pub const UART_IER_THRI: i32 = 0x02;
pub const UART_IER_RDI: i32 = 0x01;
pub const UART_IERX_SLEEP: i32 = 0x10;

pub const UART_IIR: i32 = 2;
pub const UART_IIR_NO_INT: i32 = 0x01;
pub const UART_IIR_ID: i32 = 0x0e;
pub const UART_IIR_MSI: i32 = 0x00;
pub const UART_IIR_THRI: i32 = 0x02;
pub const UART_IIR_RDI: i32 = 0x04;
pub const UART_IIR_RLSI: i32 = 0x06;
pub const UART_IIR_BUSY: i32 = 0x07;
pub const UART_IIR_RX_TIMEOUT: i32 = 0x0c;
pub const UART_IIR_XOFF: i32 = 0x10;
pub const UART_IIR_CTS_RTS_DSR: i32 = 0x20;
pub const UART_IIR_64BYTE_FIFO: i32 = 0x20;
pub const UART_IIR_FIFO_ENABLED: i32 = 0xc0;
pub const UART_IIR_FIFO_ENABLED_8250: i32 = 0x00;
pub const UART_IIR_FIFO_ENABLED_16550: i32 = 0x80;
pub const UART_IIR_FIFO_ENABLED_16550A: i32 = 0xc0;
pub const UART_IIR_FIFO_ENABLED_16750: i32 = 0xe0;

pub const UART_FCR: i32 = 2;
pub const UART_FCR_ENABLE_FIFO: i32 = 0x01;
pub const UART_FCR_CLEAR_RCVR: i32 = 0x02;
pub const UART_FCR_CLEAR_XMIT: i32 = 0x04;
pub const UART_FCR_DMA_SELECT: i32 = 0x08;
pub const UART_FCR_R_TRIG_00: i32 = 0x00;
pub const UART_FCR_R_TRIG_01: i32 = 0x40;
pub const UART_FCR_R_TRIG_10: i32 = 0x80;
pub const UART_FCR_R_TRIG_11: i32 = 0xc0;
pub const UART_FCR_T_TRIG_00: i32 = 0x00;
pub const UART_FCR_T_TRIG_01: i32 = 0x10;
pub const UART_FCR_T_TRIG_10: i32 = 0x20;
pub const UART_FCR_T_TRIG_11: i32 = 0x30;
pub const UART_FCR_TRIGGER_MASK: i32 = 0xc0;
pub const UART_FCR_TRIGGER_1: i32 = 0x00;
pub const UART_FCR_TRIGGER_4: i32 = 0x40;
pub const UART_FCR_TRIGGER_8: i32 = 0x80;
pub const UART_FCR_TRIGGER_14: i32 = 0xc0;
pub const UART_FCR6_R_TRIGGER_8: i32 = 0x00;
pub const UART_FCR6_R_TRIGGER_16: i32 = 0x40;
pub const UART_FCR6_R_TRIGGER_24: i32 = 0x80;
pub const UART_FCR6_R_TRIGGER_28: i32 = 0xc0;
pub const UART_FCR6_T_TRIGGER_16: i32 = 0x00;
pub const UART_FCR6_T_TRIGGER_8: i32 = 0x10;
pub const UART_FCR6_T_TRIGGER_24: i32 = 0x20;
pub const UART_FCR6_T_TRIGGER_30: i32 = 0x30;
pub const UART_FCR7_64BYTE: i32 = 0x20;
pub const UART_FCR_R_TRIG_SHIFT: i32 = 6;
/// C integer-promotion and usual-arithmetic-conversion mapping for
/// `UART_FCR_R_TRIG_BITS` on the frozen 32-bit-`int` architectures.
///
/// The C macro's `int` mask promotes 8- and 16-bit operands to `int`, leaves
/// a signed `int` operand signed, and converts that mask to the operand's
/// unsigned or wider type when required.
pub trait UartFcrRTrigBitsInput {
    type Output;

    fn uart_fcr_r_trig_bits(self) -> Self::Output;
}

macro_rules! impl_uart_fcr_r_trig_bits_promoted_to_i32 {
    ($($type:ty),+ $(,)?) => {
        $(
            impl UartFcrRTrigBitsInput for $type {
                type Output = i32;

                #[inline]
                fn uart_fcr_r_trig_bits(self) -> Self::Output {
                    ((self as i32) & UART_FCR_TRIGGER_MASK) >> UART_FCR_R_TRIG_SHIFT
                }
            }
        )+
    };
}

impl_uart_fcr_r_trig_bits_promoted_to_i32!(i8, u8, i16, u16, i32);

macro_rules! impl_uart_fcr_r_trig_bits_in_operand_type {
    ($($type:ty),+ $(,)?) => {
        $(
            impl UartFcrRTrigBitsInput for $type {
                type Output = $type;

                #[inline]
                fn uart_fcr_r_trig_bits(self) -> Self::Output {
                    (self & (UART_FCR_TRIGGER_MASK as $type))
                        >> (UART_FCR_R_TRIG_SHIFT as $type)
                }
            }
        )+
    };
}

impl_uart_fcr_r_trig_bits_in_operand_type!(u32, i64, u64, isize, usize);

#[inline]
pub fn UART_FCR_R_TRIG_BITS<T: UartFcrRTrigBitsInput>(x: T) -> T::Output {
    x.uart_fcr_r_trig_bits()
}
pub const UART_FCR_R_TRIG_MAX_STATE: i32 = 4;

pub const UART_LCR: i32 = 3;
pub const UART_LCR_DLAB: i32 = 0x80;
pub const UART_LCR_SBC: i32 = 0x40;
pub const UART_LCR_SPAR: i32 = 0x20;
pub const UART_LCR_EPAR: i32 = 0x10;
pub const UART_LCR_PARITY: i32 = 0x08;
pub const UART_LCR_STOP: i32 = 0x04;
pub const UART_LCR_WLEN5: i32 = 0x00;
pub const UART_LCR_WLEN6: i32 = 0x01;
pub const UART_LCR_WLEN7: i32 = 0x02;
pub const UART_LCR_WLEN8: i32 = 0x03;
pub const UART_LCR_CONF_MODE_A: i32 = UART_LCR_DLAB;
pub const UART_LCR_CONF_MODE_B: i32 = 0xbf;

pub const UART_MCR: i32 = 4;
pub const UART_MCR_CLKSEL: i32 = 0x80;
pub const UART_MCR_TCRTLR: i32 = 0x40;
pub const UART_MCR_XONANY: i32 = 0x20;
pub const UART_MCR_AFE: i32 = 0x20;
pub const UART_MCR_LOOP: i32 = 0x10;
pub const UART_MCR_OUT2: i32 = 0x08;
pub const UART_MCR_OUT1: i32 = 0x04;
pub const UART_MCR_RTS: i32 = 0x02;
pub const UART_MCR_DTR: i32 = 0x01;

pub const UART_LSR: i32 = 5;
pub const UART_LSR_FIFOE: i32 = 0x80;
pub const UART_LSR_TEMT: i32 = 0x40;
pub const UART_LSR_THRE: i32 = 0x20;
pub const UART_LSR_BI: i32 = 0x10;
pub const UART_LSR_FE: i32 = 0x08;
pub const UART_LSR_PE: i32 = 0x04;
pub const UART_LSR_OE: i32 = 0x02;
pub const UART_LSR_DR: i32 = 0x01;
pub const UART_LSR_BRK_ERROR_BITS: i32 = UART_LSR_BI | UART_LSR_FE | UART_LSR_PE | UART_LSR_OE;

pub const UART_MSR: i32 = 6;
pub const UART_MSR_DCD: i32 = 0x80;
pub const UART_MSR_RI: i32 = 0x40;
pub const UART_MSR_DSR: i32 = 0x20;
pub const UART_MSR_CTS: i32 = 0x10;
pub const UART_MSR_DDCD: i32 = 0x08;
pub const UART_MSR_TERI: i32 = 0x04;
pub const UART_MSR_DDSR: i32 = 0x02;
pub const UART_MSR_DCTS: i32 = 0x01;
pub const UART_MSR_ANY_DELTA: i32 = UART_MSR_DDCD | UART_MSR_TERI | UART_MSR_DDSR | UART_MSR_DCTS;
pub const UART_SCR: i32 = 7;

pub const UART_DLL: i32 = 0;
pub const UART_DLM: i32 = 1;
pub const UART_DIV_MAX: i32 = 0xffff;
pub const UART_EFR: i32 = 2;
pub const UART_XR_EFR: i32 = 9;
pub const UART_EFR_CTS: i32 = 0x80;
pub const UART_EFR_RTS: i32 = 0x40;
pub const UART_EFR_SCD: i32 = 0x20;
pub const UART_EFR_ECB: i32 = 0x10;
pub const UART_XON1: i32 = 4;
pub const UART_XON2: i32 = 5;
pub const UART_XOFF1: i32 = 6;
pub const UART_XOFF2: i32 = 7;
pub const UART_TI752_TCR: i32 = 6;
pub const UART_TI752_TLR: i32 = 7;

pub const UART_TRG: i32 = 0;
pub const UART_TRG_1: i32 = 0x01;
pub const UART_TRG_4: i32 = 0x04;
pub const UART_TRG_8: i32 = 0x08;
pub const UART_TRG_16: i32 = 0x10;
pub const UART_TRG_32: i32 = 0x20;
pub const UART_TRG_64: i32 = 0x40;
pub const UART_TRG_96: i32 = 0x60;
pub const UART_TRG_120: i32 = 0x78;
pub const UART_TRG_128: i32 = 0x80;
pub const UART_FCTR: i32 = 1;
pub const UART_FCTR_RTS_NODELAY: i32 = 0x00;
pub const UART_FCTR_RTS_4DELAY: i32 = 0x01;
pub const UART_FCTR_RTS_6DELAY: i32 = 0x02;
pub const UART_FCTR_RTS_8DELAY: i32 = 0x03;
pub const UART_FCTR_IRDA: i32 = 0x04;
pub const UART_FCTR_TX_INT: i32 = 0x08;
pub const UART_FCTR_TRGA: i32 = 0x00;
pub const UART_FCTR_TRGB: i32 = 0x10;
pub const UART_FCTR_TRGC: i32 = 0x20;
pub const UART_FCTR_TRGD: i32 = 0x30;
pub const UART_FCTR_SCR_SWAP: i32 = 0x40;
pub const UART_FCTR_RX: i32 = 0x00;
pub const UART_FCTR_TX: i32 = 0x80;
pub const UART_EMSR: i32 = 7;
pub const UART_EMSR_FIFO_COUNT: i32 = 0x01;
pub const UART_EMSR_ALT_COUNT: i32 = 0x02;

pub const UART_IER_DMAE: i32 = 0x80;
pub const UART_IER_UUE: i32 = 0x40;
pub const UART_IER_NRZE: i32 = 0x20;
pub const UART_IER_RTOIE: i32 = 0x10;
pub const UART_IIR_TOD: i32 = 0x08;
pub const UART_FCR_PXAR1: i32 = 0x00;
pub const UART_FCR_PXAR8: i32 = 0x40;
pub const UART_FCR_PXAR16: i32 = 0x80;
pub const UART_FCR_PXAR32: i32 = 0xc0;

pub const UART_ASR: i32 = 0x01;
pub const UART_RFL: i32 = 0x03;
pub const UART_TFL: i32 = 0x04;
pub const UART_ICR: i32 = 0x05;
pub const UART_ACR: i32 = 0x00;
pub const UART_CPR: i32 = 0x01;
pub const UART_TCR: i32 = 0x02;
pub const UART_CKS: i32 = 0x03;
pub const UART_TTL: i32 = 0x04;
pub const UART_RTL: i32 = 0x05;
pub const UART_FCL: i32 = 0x06;
pub const UART_FCH: i32 = 0x07;
pub const UART_ID1: i32 = 0x08;
pub const UART_ID2: i32 = 0x09;
pub const UART_ID3: i32 = 0x0a;
pub const UART_REV: i32 = 0x0b;
pub const UART_CSR: i32 = 0x0c;
pub const UART_NMR: i32 = 0x0d;
pub const UART_CTR: i32 = 0xff;
pub const UART_ACR_RXDIS: i32 = 0x01;
pub const UART_ACR_TXDIS: i32 = 0x02;
pub const UART_ACR_DSRFC: i32 = 0x04;
pub const UART_ACR_TLENB: i32 = 0x20;
pub const UART_ACR_ICRRD: i32 = 0x40;
pub const UART_ACR_ASREN: i32 = 0x80;

pub const UART_RSA_BASE: i32 = -8;
pub const UART_RSA_MSR: i32 = UART_RSA_BASE + 0;
pub const UART_RSA_MSR_SWAP: i32 = 1 << 0;
pub const UART_RSA_MSR_FIFO: i32 = 1 << 2;
pub const UART_RSA_MSR_FLOW: i32 = 1 << 3;
pub const UART_RSA_MSR_ITYP: i32 = 1 << 4;
pub const UART_RSA_IER: i32 = UART_RSA_BASE + 1;
pub const UART_RSA_IER_Rx_FIFO_H: i32 = 1 << 0;
pub const UART_RSA_IER_Tx_FIFO_H: i32 = 1 << 1;
pub const UART_RSA_IER_Tx_FIFO_E: i32 = 1 << 2;
pub const UART_RSA_IER_Rx_TOUT: i32 = 1 << 3;
pub const UART_RSA_IER_TIMER: i32 = 1 << 4;
pub const UART_RSA_SRR: i32 = UART_RSA_BASE + 2;
pub const UART_RSA_SRR_Tx_FIFO_NEMP: i32 = 1 << 0;
pub const UART_RSA_SRR_Tx_FIFO_NHFL: i32 = 1 << 1;
pub const UART_RSA_SRR_Tx_FIFO_NFUL: i32 = 1 << 2;
pub const UART_RSA_SRR_Rx_FIFO_NEMP: i32 = 1 << 3;
pub const UART_RSA_SRR_Rx_FIFO_NHFL: i32 = 1 << 4;
pub const UART_RSA_SRR_Rx_FIFO_NFUL: i32 = 1 << 5;
pub const UART_RSA_SRR_Rx_TOUT: i32 = 1 << 6;
pub const UART_RSA_SRR_TIMER: i32 = 1 << 7;
pub const UART_RSA_FRR: i32 = UART_RSA_BASE + 2;
pub const UART_RSA_TIVSR: i32 = UART_RSA_BASE + 3;
pub const UART_RSA_TCR: i32 = UART_RSA_BASE + 4;
pub const UART_RSA_TCR_SWITCH: i32 = 1 << 0;
pub const SERIAL_RSA_BAUD_BASE: i32 = 921600;
pub const SERIAL_RSA_BAUD_BASE_LO: i32 = SERIAL_RSA_BAUD_BASE / 8;

pub const UART_DA830_PWREMU_MGMT: i32 = 12;
pub const UART_DA830_PWREMU_MGMT_FREE: i32 = 1 << 0;
pub const UART_DA830_PWREMU_MGMT_URRST: i32 = 1 << 13;
pub const UART_DA830_PWREMU_MGMT_UTRST: i32 = 1 << 14;

pub const OMAP1_UART1_BASE: u32 = 0xfffb0000;
pub const OMAP1_UART2_BASE: u32 = 0xfffb0800;
pub const OMAP1_UART3_BASE: u32 = 0xfffb9800;
pub const UART_OMAP_MDR1: i32 = 0x08;
pub const UART_OMAP_MDR2: i32 = 0x09;
pub const UART_OMAP_SCR: i32 = 0x10;
pub const UART_OMAP_SSR: i32 = 0x11;
pub const UART_OMAP_EBLR: i32 = 0x12;
pub const UART_OMAP_OSC_12M_SEL: i32 = 0x13;
pub const UART_OMAP_MVER: i32 = 0x14;
pub const UART_OMAP_SYSC: i32 = 0x15;
pub const UART_OMAP_SYSS: i32 = 0x16;
pub const UART_OMAP_WER: i32 = 0x17;
pub const UART_OMAP_TX_LVL: i32 = 0x1a;
pub const UART_OMAP_MDR1_16X_MODE: i32 = 0x00;
pub const UART_OMAP_MDR1_SIR_MODE: i32 = 0x01;
pub const UART_OMAP_MDR1_16X_ABAUD_MODE: i32 = 0x02;
pub const UART_OMAP_MDR1_13X_MODE: i32 = 0x03;
pub const UART_OMAP_MDR1_MIR_MODE: i32 = 0x04;
pub const UART_OMAP_MDR1_FIR_MODE: i32 = 0x05;
pub const UART_OMAP_MDR1_CIR_MODE: i32 = 0x06;
pub const UART_OMAP_MDR1_DISABLE: i32 = 0x07;
pub const UART_ALTR_AFR: i32 = 0x40;
pub const UART_ALTR_EN_TXFIFO_LW: i32 = 0x01;
pub const UART_ALTR_TX_LOW: i32 = 0x41;
