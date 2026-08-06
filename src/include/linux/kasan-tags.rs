// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/linux/kasan-tags.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014160

/// Native kernel pointers tag.
///
/// This preserves the type of the unsuffixed C integer literal in
/// `KASAN_TAG_KERNEL`.
pub const KASAN_TAG_KERNEL: i32 = 0xFF;

/// Inaccessible memory tag.
///
/// This preserves the type of the unsuffixed C integer literal in
/// `KASAN_TAG_INVALID`.
pub const KASAN_TAG_INVALID: i32 = 0xFE;

/// Maximum value for random tags.
///
/// This preserves the type of the unsuffixed C integer literal in
/// `KASAN_TAG_MAX`.
pub const KASAN_TAG_MAX: i32 = 0xFD;

/// Minimum value for random tags in the frozen configuration union.
///
/// Both selected configurations leave `CONFIG_KASAN_HW_TAGS` undefined, so
/// the Linux header selects its `#else` definition (`0x00`), rather than the
/// hardware-tag definition (`0xF0`).
pub const KASAN_TAG_MIN: i32 = 0x00;
