// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/virtio_ids.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016464

/*
 * Virtio IDs
 *
 * This header is BSD licensed so anyone can use the definitions to implement
 * compatible drivers/servers.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of IBM nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL IBM OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE. */

use core::ffi::c_int;

pub const VIRTIO_ID_NET: c_int = 1;
pub const VIRTIO_ID_BLOCK: c_int = 2;
pub const VIRTIO_ID_CONSOLE: c_int = 3;
pub const VIRTIO_ID_RNG: c_int = 4;
pub const VIRTIO_ID_BALLOON: c_int = 5;
pub const VIRTIO_ID_IOMEM: c_int = 6;
pub const VIRTIO_ID_RPMSG: c_int = 7;
pub const VIRTIO_ID_SCSI: c_int = 8;
pub const VIRTIO_ID_9P: c_int = 9;
pub const VIRTIO_ID_MAC80211_WLAN: c_int = 10;
pub const VIRTIO_ID_RPROC_SERIAL: c_int = 11;
pub const VIRTIO_ID_CAIF: c_int = 12;
pub const VIRTIO_ID_MEMORY_BALLOON: c_int = 13;
pub const VIRTIO_ID_GPU: c_int = 16;
pub const VIRTIO_ID_CLOCK: c_int = 17;
pub const VIRTIO_ID_INPUT: c_int = 18;
pub const VIRTIO_ID_VSOCK: c_int = 19;
pub const VIRTIO_ID_CRYPTO: c_int = 20;
pub const VIRTIO_ID_SIGNAL_DIST: c_int = 21;
pub const VIRTIO_ID_PSTORE: c_int = 22;
pub const VIRTIO_ID_IOMMU: c_int = 23;
pub const VIRTIO_ID_MEM: c_int = 24;
pub const VIRTIO_ID_SOUND: c_int = 25;
pub const VIRTIO_ID_FS: c_int = 26;
pub const VIRTIO_ID_PMEM: c_int = 27;
pub const VIRTIO_ID_RPMB: c_int = 28;
pub const VIRTIO_ID_MAC80211_HWSIM: c_int = 29;
pub const VIRTIO_ID_VIDEO_ENCODER: c_int = 30;
pub const VIRTIO_ID_VIDEO_DECODER: c_int = 31;
pub const VIRTIO_ID_SCMI: c_int = 32;
pub const VIRTIO_ID_NITRO_SEC_MOD: c_int = 33;
pub const VIRTIO_ID_I2C_ADAPTER: c_int = 34;
pub const VIRTIO_ID_WATCHDOG: c_int = 35;
pub const VIRTIO_ID_CAN: c_int = 36;
pub const VIRTIO_ID_DMABUF: c_int = 37;
pub const VIRTIO_ID_PARAM_SERV: c_int = 38;
pub const VIRTIO_ID_AUDIO_POLICY: c_int = 39;
pub const VIRTIO_ID_BT: c_int = 40;
pub const VIRTIO_ID_GPIO: c_int = 41;
pub const VIRTIO_ID_SPI: c_int = 45;

pub const VIRTIO_TRANS_ID_NET: c_int = 0x1000;
pub const VIRTIO_TRANS_ID_BLOCK: c_int = 0x1001;
pub const VIRTIO_TRANS_ID_BALLOON: c_int = 0x1002;
pub const VIRTIO_TRANS_ID_CONSOLE: c_int = 0x1003;
pub const VIRTIO_TRANS_ID_SCSI: c_int = 0x1004;
pub const VIRTIO_TRANS_ID_RNG: c_int = 0x1005;
pub const VIRTIO_TRANS_ID_9P: c_int = 0x1009;
