// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/nfs_iostat.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014514

pub const NFS_IOSTAT_VERS: &str = "1.1";

pub type nfs_stat_bytecounters = i32;

pub const NFSIOS_NORMALREADBYTES: nfs_stat_bytecounters = 0;
pub const NFSIOS_NORMALWRITTENBYTES: nfs_stat_bytecounters = 1;
pub const NFSIOS_DIRECTREADBYTES: nfs_stat_bytecounters = 2;
pub const NFSIOS_DIRECTWRITTENBYTES: nfs_stat_bytecounters = 3;
pub const NFSIOS_SERVERREADBYTES: nfs_stat_bytecounters = 4;
pub const NFSIOS_SERVERWRITTENBYTES: nfs_stat_bytecounters = 5;
pub const NFSIOS_READPAGES: nfs_stat_bytecounters = 6;
pub const NFSIOS_WRITEPAGES: nfs_stat_bytecounters = 7;
pub const __NFSIOS_BYTESMAX: nfs_stat_bytecounters = 8;

pub type nfs_stat_eventcounters = i32;

pub const NFSIOS_INODEREVALIDATE: nfs_stat_eventcounters = 0;
pub const NFSIOS_DENTRYREVALIDATE: nfs_stat_eventcounters = 1;
pub const NFSIOS_DATAINVALIDATE: nfs_stat_eventcounters = 2;
pub const NFSIOS_ATTRINVALIDATE: nfs_stat_eventcounters = 3;
pub const NFSIOS_VFSOPEN: nfs_stat_eventcounters = 4;
pub const NFSIOS_VFSLOOKUP: nfs_stat_eventcounters = 5;
pub const NFSIOS_VFSACCESS: nfs_stat_eventcounters = 6;
pub const NFSIOS_VFSUPDATEPAGE: nfs_stat_eventcounters = 7;
pub const NFSIOS_VFSREADPAGE: nfs_stat_eventcounters = 8;
pub const NFSIOS_VFSREADPAGES: nfs_stat_eventcounters = 9;
pub const NFSIOS_VFSWRITEPAGE: nfs_stat_eventcounters = 10;
pub const NFSIOS_VFSWRITEPAGES: nfs_stat_eventcounters = 11;
pub const NFSIOS_VFSGETDENTS: nfs_stat_eventcounters = 12;
pub const NFSIOS_VFSSETATTR: nfs_stat_eventcounters = 13;
pub const NFSIOS_VFSFLUSH: nfs_stat_eventcounters = 14;
pub const NFSIOS_VFSFSYNC: nfs_stat_eventcounters = 15;
pub const NFSIOS_VFSLOCK: nfs_stat_eventcounters = 16;
pub const NFSIOS_VFSRELEASE: nfs_stat_eventcounters = 17;
pub const NFSIOS_CONGESTIONWAIT: nfs_stat_eventcounters = 18;
pub const NFSIOS_SETATTRTRUNC: nfs_stat_eventcounters = 19;
pub const NFSIOS_EXTENDWRITE: nfs_stat_eventcounters = 20;
pub const NFSIOS_SILLYRENAME: nfs_stat_eventcounters = 21;
pub const NFSIOS_SHORTREAD: nfs_stat_eventcounters = 22;
pub const NFSIOS_SHORTWRITE: nfs_stat_eventcounters = 23;
pub const NFSIOS_DELAY: nfs_stat_eventcounters = 24;
pub const NFSIOS_PNFS_READ: nfs_stat_eventcounters = 25;
pub const NFSIOS_PNFS_WRITE: nfs_stat_eventcounters = 26;
pub const __NFSIOS_COUNTSMAX: nfs_stat_eventcounters = 27;
