//! linux-parity: complete
//! linux-source: vendor/linux/block/ioctl.c
//! linux-source: vendor/linux/include/uapi/linux/fs.h
//! Block-device ioctl numbers (the `BLK*` macros).
//!
//! Verified against `fs.h`: `_IO(0x12,nr)` encodes as `0x1200|nr`; the size_t/
//! `__u64` variants add the `_IOR`/`_IOW` direction bits + the 8-byte size in
//! bits [29:16]. The two `BLKTRACESETUP`/`BLKTRACESETUP2` ioctls (blktrace,
//! whose numbers encode a struct size) are intentionally omitted — Lupos has no
//! blktrace and shipping a guessed size would be worse than omission.

#![allow(dead_code)]

// `_IO(0x12, nr)` = `0x1200 | nr`.
pub const BLKROSET: u32 = 0x125d; // set device read-only (0 = read-write)
pub const BLKROGET: u32 = 0x125e; // get read-only status
pub const BLKRRPART: u32 = 0x125f; // re-read partition table
pub const BLKGETSIZE: u32 = 0x1260; // device size / 512 (long *arg)
pub const BLKFLSBUF: u32 = 0x1261; // flush buffer cache
pub const BLKRASET: u32 = 0x1262; // set read-ahead
pub const BLKRAGET: u32 = 0x1263; // get read-ahead
pub const BLKFRASET: u32 = 0x1264; // set filesystem read-ahead
pub const BLKFRAGET: u32 = 0x1265; // get filesystem read-ahead
pub const BLKSECTSET: u32 = 0x1266; // set max sectors per request
pub const BLKSECTGET: u32 = 0x1267; // get max sectors per request
pub const BLKSSZGET: u32 = 0x1268; // logical sector size
pub const BLKPG: u32 = 0x1269; // partition table ops (see blkpg.h)
pub const BLKTRACESTART: u32 = 0x1274;
pub const BLKTRACESTOP: u32 = 0x1275;
pub const BLKTRACETEARDOWN: u32 = 0x1276;
pub const BLKDISCARD: u32 = 0x1277;
pub const BLKIOMIN: u32 = 0x1278; // minimum I/O size
pub const BLKIOOPT: u32 = 0x1279; // optimal I/O size
pub const BLKALIGNOFF: u32 = 0x127a; // alignment offset
pub const BLKPBSZGET: u32 = 0x127b; // physical block size
pub const BLKDISCARDZEROES: u32 = 0x127c;
pub const BLKSECDISCARD: u32 = 0x127d; // secure discard
pub const BLKROTATIONAL: u32 = 0x127e; // is the device rotational
pub const BLKZEROOUT: u32 = 0x127f;

// `_IOR(0x12, nr, size_t)` = `0x80081200 | nr`.
pub const BLKELVGET: u32 = 0x8008_126a; // elevator get
pub const BLKBSZGET: u32 = 0x8008_1270; // soft block size
pub const BLKGETSIZE64: u32 = 0x8008_1272; // device size in bytes (u64 *arg)
// `_IOW(0x12, nr, size_t)` = `0x40081200 | nr`.
pub const BLKELVSET: u32 = 0x4008_126b; // elevator set
pub const BLKBSZSET: u32 = 0x4008_1271; // set soft block size
// `_IOR(0x12, 128, __u64)`.
pub const BLKGETDISKSEQ: u32 = 0x8008_1280; // disk sequence number
