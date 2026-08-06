// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
//! linux-source: include/uapi/linux/vesa.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016454

/* VESA Blanking Levels */
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct vesa_blank_mode(pub i32);

impl vesa_blank_mode {
    pub const VESA_NO_BLANKING: Self = Self(0);
    pub const VESA_VSYNC_SUSPEND: Self = Self(1);
    pub const VESA_HSYNC_SUSPEND: Self = Self(2);
    pub const VESA_POWERDOWN: Self = Self(
        Self::VESA_VSYNC_SUSPEND.0 | Self::VESA_HSYNC_SUSPEND.0,
    );
    pub const VESA_BLANK_MAX: Self = Self(Self::VESA_POWERDOWN.0);
}

pub const VESA_NO_BLANKING: i32 = vesa_blank_mode::VESA_NO_BLANKING.0;
pub const VESA_VSYNC_SUSPEND: i32 = vesa_blank_mode::VESA_VSYNC_SUSPEND.0;
pub const VESA_HSYNC_SUSPEND: i32 = vesa_blank_mode::VESA_HSYNC_SUSPEND.0;
pub const VESA_POWERDOWN: i32 = vesa_blank_mode::VESA_POWERDOWN.0;
pub const VESA_BLANK_MAX: i32 = vesa_blank_mode::VESA_BLANK_MAX.0;
