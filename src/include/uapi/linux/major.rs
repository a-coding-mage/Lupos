// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/major.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016234

//! Major device-number definitions.

pub const UNNAMED_MAJOR: i32 = 0;
pub const MEM_MAJOR: i32 = 1;
pub const RAMDISK_MAJOR: i32 = 1;
pub const FLOPPY_MAJOR: i32 = 2;
pub const PTY_MASTER_MAJOR: i32 = 2;
pub const IDE0_MAJOR: i32 = 3;
pub const HD_MAJOR: i32 = IDE0_MAJOR;
pub const PTY_SLAVE_MAJOR: i32 = 3;
pub const TTY_MAJOR: i32 = 4;
pub const TTYAUX_MAJOR: i32 = 5;
pub const LP_MAJOR: i32 = 6;
pub const VCS_MAJOR: i32 = 7;
pub const LOOP_MAJOR: i32 = 7;
pub const SCSI_DISK0_MAJOR: i32 = 8;
pub const SCSI_TAPE_MAJOR: i32 = 9;
pub const MD_MAJOR: i32 = 9;
pub const MISC_MAJOR: i32 = 10;
pub const SCSI_CDROM_MAJOR: i32 = 11;
pub const MUX_MAJOR: i32 = 11;
pub const XT_DISK_MAJOR: i32 = 13;
pub const INPUT_MAJOR: i32 = 13;
pub const SOUND_MAJOR: i32 = 14;
pub const CDU31A_CDROM_MAJOR: i32 = 15;
pub const JOYSTICK_MAJOR: i32 = 15;
pub const GOLDSTAR_CDROM_MAJOR: i32 = 16;
pub const OPTICS_CDROM_MAJOR: i32 = 17;
pub const SANYO_CDROM_MAJOR: i32 = 18;
pub const MITSUMI_X_CDROM_MAJOR: i32 = 20;
pub const MFM_ACORN_MAJOR: i32 = 21;
pub const SCSI_GENERIC_MAJOR: i32 = 21;
pub const IDE1_MAJOR: i32 = 22;
pub const DIGICU_MAJOR: i32 = 22;
pub const DIGI_MAJOR: i32 = 23;
pub const MITSUMI_CDROM_MAJOR: i32 = 23;
pub const CDU535_CDROM_MAJOR: i32 = 24;
pub const STL_SERIALMAJOR: i32 = 24;
pub const MATSUSHITA_CDROM_MAJOR: i32 = 25;
pub const STL_CALLOUTMAJOR: i32 = 25;
pub const MATSUSHITA_CDROM2_MAJOR: i32 = 26;
pub const QIC117_TAPE_MAJOR: i32 = 27;
pub const MATSUSHITA_CDROM3_MAJOR: i32 = 27;
pub const MATSUSHITA_CDROM4_MAJOR: i32 = 28;
pub const STL_SIOMEMMAJOR: i32 = 28;
pub const ACSI_MAJOR: i32 = 28;
pub const AZTECH_CDROM_MAJOR: i32 = 29;
pub const FB_MAJOR: i32 = 29;
pub const MTD_BLOCK_MAJOR: i32 = 31;
pub const CM206_CDROM_MAJOR: i32 = 32;
pub const IDE2_MAJOR: i32 = 33;
pub const IDE3_MAJOR: i32 = 34;
pub const Z8530_MAJOR: i32 = 34;
pub const XPRAM_MAJOR: i32 = 35;
pub const NETLINK_MAJOR: i32 = 36;
pub const PS2ESDI_MAJOR: i32 = 36;
pub const IDETAPE_MAJOR: i32 = 37;
pub const Z2RAM_MAJOR: i32 = 37;
pub const APBLOCK_MAJOR: i32 = 38;
pub const DDV_MAJOR: i32 = 39;
pub const NBD_MAJOR: i32 = 43;
pub const RISCOM8_NORMAL_MAJOR: i32 = 48;
pub const DAC960_MAJOR: i32 = 48;
pub const RISCOM8_CALLOUT_MAJOR: i32 = 49;
pub const MKISS_MAJOR: i32 = 55;
pub const DSP56K_MAJOR: i32 = 55;

pub const IDE4_MAJOR: i32 = 56;
pub const IDE5_MAJOR: i32 = 57;

pub const SCSI_DISK1_MAJOR: i32 = 65;
pub const SCSI_DISK2_MAJOR: i32 = 66;
pub const SCSI_DISK3_MAJOR: i32 = 67;
pub const SCSI_DISK4_MAJOR: i32 = 68;
pub const SCSI_DISK5_MAJOR: i32 = 69;
pub const SCSI_DISK6_MAJOR: i32 = 70;
pub const SCSI_DISK7_MAJOR: i32 = 71;

pub const COMPAQ_SMART2_MAJOR: i32 = 72;
pub const COMPAQ_SMART2_MAJOR1: i32 = 73;
pub const COMPAQ_SMART2_MAJOR2: i32 = 74;
pub const COMPAQ_SMART2_MAJOR3: i32 = 75;
pub const COMPAQ_SMART2_MAJOR4: i32 = 76;
pub const COMPAQ_SMART2_MAJOR5: i32 = 77;
pub const COMPAQ_SMART2_MAJOR6: i32 = 78;
pub const COMPAQ_SMART2_MAJOR7: i32 = 79;

pub const SPECIALIX_NORMAL_MAJOR: i32 = 75;
pub const SPECIALIX_CALLOUT_MAJOR: i32 = 76;

pub const AURORA_MAJOR: i32 = 79;

pub const I2O_MAJOR: i32 = 80;

pub const SHMIQ_MAJOR: i32 = 85;
pub const SCSI_CHANGER_MAJOR: i32 = 86;

pub const IDE6_MAJOR: i32 = 88;
pub const IDE7_MAJOR: i32 = 89;
pub const IDE8_MAJOR: i32 = 90;
pub const MTD_CHAR_MAJOR: i32 = 90;
pub const IDE9_MAJOR: i32 = 91;

pub const DASD_MAJOR: i32 = 94;

pub const MDISK_MAJOR: i32 = 95;

pub const UBD_MAJOR: i32 = 98;

pub const PP_MAJOR: i32 = 99;
pub const JSFD_MAJOR: i32 = 99;

pub const PHONE_MAJOR: i32 = 100;

pub const COMPAQ_CISS_MAJOR: i32 = 104;
pub const COMPAQ_CISS_MAJOR1: i32 = 105;
pub const COMPAQ_CISS_MAJOR2: i32 = 106;
pub const COMPAQ_CISS_MAJOR3: i32 = 107;
pub const COMPAQ_CISS_MAJOR4: i32 = 108;
pub const COMPAQ_CISS_MAJOR5: i32 = 109;
pub const COMPAQ_CISS_MAJOR6: i32 = 110;
pub const COMPAQ_CISS_MAJOR7: i32 = 111;

pub const VIODASD_MAJOR: i32 = 112;
pub const VIOCD_MAJOR: i32 = 113;

pub const ATARAID_MAJOR: i32 = 114;

pub const SCSI_DISK8_MAJOR: i32 = 128;
pub const SCSI_DISK9_MAJOR: i32 = 129;
pub const SCSI_DISK10_MAJOR: i32 = 130;
pub const SCSI_DISK11_MAJOR: i32 = 131;
pub const SCSI_DISK12_MAJOR: i32 = 132;
pub const SCSI_DISK13_MAJOR: i32 = 133;
pub const SCSI_DISK14_MAJOR: i32 = 134;
pub const SCSI_DISK15_MAJOR: i32 = 135;

pub const UNIX98_PTY_MASTER_MAJOR: i32 = 128;
pub const UNIX98_PTY_MAJOR_COUNT: i32 = 8;
pub const UNIX98_PTY_SLAVE_MAJOR: i32 = (UNIX98_PTY_MASTER_MAJOR + UNIX98_PTY_MAJOR_COUNT);

pub const DRBD_MAJOR: i32 = 147;
pub const RTF_MAJOR: i32 = 150;
pub const RAW_MAJOR: i32 = 162;

pub const USB_ACM_MAJOR: i32 = 166;
pub const USB_ACM_AUX_MAJOR: i32 = 167;
pub const USB_CHAR_MAJOR: i32 = 180;

pub const MMC_BLOCK_MAJOR: i32 = 179;

pub const VXVM_MAJOR: i32 = 199;
pub const VXSPEC_MAJOR: i32 = 200;
pub const VXDMP_MAJOR: i32 = 201;

pub const XENVBD_MAJOR: i32 = 202;

pub const MSR_MAJOR: i32 = 202;
pub const CPUID_MAJOR: i32 = 203;

pub const OSST_MAJOR: i32 = 206;

pub const IBM_TTY3270_MAJOR: i32 = 227;
pub const IBM_FS3270_MAJOR: i32 = 228;

pub const VIOTAPE_MAJOR: i32 = 230;

pub const BLOCK_EXT_MAJOR: i32 = 259;
pub const SCSI_OSD_MAJOR: i32 = 260;
