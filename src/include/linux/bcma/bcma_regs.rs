// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/bcma/bcma_regs.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013505

/* Some single registers are shared between many cores. */
/* BCMA_CLKCTLST: ChipCommon (rev >= 20), PCIe, 80211. */
pub const BCMA_CLKCTLST: i32 = 0x01e0; /* Clock control and status */
pub const BCMA_CLKCTLST_FORCEALP: i32 = 0x00000001; /* Force ALP request */
pub const BCMA_CLKCTLST_FORCEHT: i32 = 0x00000002; /* Force HT request */
pub const BCMA_CLKCTLST_FORCEILP: i32 = 0x00000004; /* Force ILP request */
pub const BCMA_CLKCTLST_HAVEALPREQ: i32 = 0x00000008; /* ALP available request */
pub const BCMA_CLKCTLST_HAVEHTREQ: i32 = 0x00000010; /* HT available request */
pub const BCMA_CLKCTLST_HWCROFF: i32 = 0x00000020; /* Force HW clock request off */
pub const BCMA_CLKCTLST_HQCLKREQ: i32 = 0x00000040; /* HQ Clock */
pub const BCMA_CLKCTLST_EXTRESREQ: i32 = 0x00000700; /* Mask of external resource requests */
pub const BCMA_CLKCTLST_EXTRESREQ_SHIFT: i32 = 8;
pub const BCMA_CLKCTLST_HAVEALP: i32 = 0x00010000; /* ALP available */
pub const BCMA_CLKCTLST_HAVEHT: i32 = 0x00020000; /* HT available */
pub const BCMA_CLKCTLST_BP_ON_ALP: i32 = 0x00040000; /* RO: running on ALP clock */
pub const BCMA_CLKCTLST_BP_ON_HT: i32 = 0x00080000; /* RO: running on HT clock */
pub const BCMA_CLKCTLST_EXTRESST: i32 = 0x07000000; /* Mask of external resource status */
pub const BCMA_CLKCTLST_EXTRESST_SHIFT: i32 = 24;
/* Is there any BCM4328 on BCMA bus? */
pub const BCMA_CLKCTLST_4328A0_HAVEHT: i32 = 0x00010000; /* 4328a0 has reversed bits */
pub const BCMA_CLKCTLST_4328A0_HAVEALP: i32 = 0x00020000; /* 4328a0 has reversed bits */

/* Agent registers (common for every core). */
pub const BCMA_OOB_SEL_OUT_A30: i32 = 0x0100;
pub const BCMA_IOCTL: i32 = 0x0408; /* IO control */
pub const BCMA_IOCTL_CLK: i32 = 0x0001;
pub const BCMA_IOCTL_FGC: i32 = 0x0002;
pub const BCMA_IOCTL_CORE_BITS: i32 = 0x3ffc;
pub const BCMA_IOCTL_PME_EN: i32 = 0x4000;
pub const BCMA_IOCTL_BIST_EN: i32 = 0x8000;
pub const BCMA_IOST: i32 = 0x0500; /* IO status */
pub const BCMA_IOST_CORE_BITS: i32 = 0x0fff;
pub const BCMA_IOST_DMA64: i32 = 0x1000;
pub const BCMA_IOST_GATED_CLK: i32 = 0x2000;
pub const BCMA_IOST_BIST_ERROR: i32 = 0x4000;
pub const BCMA_IOST_BIST_DONE: i32 = 0x8000;
pub const BCMA_RESET_CTL: i32 = 0x0800;
pub const BCMA_RESET_CTL_RESET: i32 = 0x0001;
pub const BCMA_RESET_ST: i32 = 0x0804;

pub const BCMA_NS_ROM_IOST_BOOT_DEV_MASK: i32 = 0x0003;
pub const BCMA_NS_ROM_IOST_BOOT_DEV_NOR: i32 = 0x0000;
pub const BCMA_NS_ROM_IOST_BOOT_DEV_NAND: i32 = 0x0001;
pub const BCMA_NS_ROM_IOST_BOOT_DEV_ROM: i32 = 0x0002;

/* BCMA PCI config space registers. */
pub const BCMA_PCI_PMCSR: i32 = 0x44;
pub const BCMA_PCI_PE: i32 = 0x100;
pub const BCMA_PCI_BAR0_WIN: i32 = 0x80; /* Backplane address space 0 */
pub const BCMA_PCI_BAR1_WIN: i32 = 0x84; /* Backplane address space 1 */
pub const BCMA_PCI_SPROMCTL: i32 = 0x88; /* SPROM control */
pub const BCMA_PCI_SPROMCTL_WE: i32 = 0x10; /* SPROM write enable */
pub const BCMA_PCI_BAR1_CONTROL: i32 = 0x8c; /* Address space 1 burst control */
pub const BCMA_PCI_IRQS: i32 = 0x90; /* PCI interrupts */
pub const BCMA_PCI_IRQMASK: i32 = 0x94; /* PCI IRQ control and mask (pcirev >= 6 only) */
pub const BCMA_PCI_BACKPLANE_IRQS: i32 = 0x98; /* Backplane Interrupts */
pub const BCMA_PCI_BAR0_WIN2: i32 = 0xac;
pub const BCMA_PCI_GPIO_IN: i32 = 0xb0; /* GPIO Input (pcirev >= 3 only) */
pub const BCMA_PCI_GPIO_OUT: i32 = 0xb4; /* GPIO Output (pcirev >= 3 only) */
pub const BCMA_PCI_GPIO_OUT_ENABLE: i32 = 0xb8; /* GPIO Output Enable/Disable (pcirev >= 3 only) */
pub const BCMA_PCI_GPIO_SCS: i32 = 0x10; /* PCI config space bit 4 for 4306c0 slow clock source */
pub const BCMA_PCI_GPIO_HWRAD: i32 = 0x20; /* PCI config space GPIO 13 for hw radio disable */
pub const BCMA_PCI_GPIO_XTAL: i32 = 0x40; /* PCI config space GPIO 14 for Xtal powerup */
pub const BCMA_PCI_GPIO_PLL: i32 = 0x80; /* PCI config space GPIO 15 for PLL powerdown */

pub const BCMA_PCIE2_BAR0_WIN2: i32 = 0x70;

/* SiliconBackplane Address Map. All regions may not exist on all chips. */
pub const BCMA_SOC_SDRAM_BASE: u32 = 0x00000000; /* Physical SDRAM */
pub const BCMA_SOC_PCI_MEM: u32 = 0x08000000; /* Host Mode sb2pcitranslation0 (64 MB) */
pub const BCMA_SOC_PCI_MEM_SZ: i32 = 64 * 1024 * 1024;
pub const BCMA_SOC_PCI_CFG: u32 = 0x0c000000; /* Host Mode sb2pcitranslation1 (64 MB) */
pub const BCMA_SOC_SDRAM_SWAPPED: u32 = 0x10000000; /* Byteswapped Physical SDRAM */
pub const BCMA_SOC_SDRAM_R2: u32 = 0x80000000; /* Region 2 for sdram (512 MB) */

pub const BCMA_SOC_PCI_DMA: u32 = 0x40000000; /* Client Mode sb2pcitranslation2 (1 GB) */
pub const BCMA_SOC_PCI_DMA2: u32 = 0x80000000; /* Client Mode sb2pcitranslation2 (1 GB) */
pub const BCMA_SOC_PCI_DMA_SZ: u32 = 0x40000000; /* Client Mode sb2pcitranslation2 size in bytes */
pub const BCMA_SOC_PCIE_DMA_L32: u32 = 0x00000000; /* PCIE Client Mode sb2pcitranslation2, low 32 bits */
pub const BCMA_SOC_PCIE_DMA_H32: u32 = 0x80000000; /* PCIE Client Mode sb2pcitranslation2, high 32 bits */

pub const BCMA_SOC_PCI1_MEM: u32 = 0x40000000; /* Host Mode sb2pcitranslation0 (64 MB) */
pub const BCMA_SOC_PCI1_CFG: u32 = 0x44000000; /* Host Mode sb2pcitranslation1 (64 MB) */
pub const BCMA_SOC_PCIE1_DMA_H32: u32 = 0xc0000000; /* PCIE Client Mode sb2pcitranslation2, high 32 bits */

pub const BCMA_SOC_FLASH1: i32 = 0x1fc00000; /* MIPS Flash Region 1 */
pub const BCMA_SOC_FLASH1_SZ: i32 = 0x00400000; /* MIPS Size of Flash Region 1 */
pub const BCMA_SOC_FLASH2: i32 = 0x1c000000; /* Flash Region 2 (region 1 shadowed here) */
pub const BCMA_SOC_FLASH2_SZ: i32 = 0x02000000; /* Size of Flash Region 2 */
