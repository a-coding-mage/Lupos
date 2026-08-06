// SPDX-License-Identifier: GPL-2.0+ WITH Linux-syscall-note
//! linux-source: include/uapi/linux/hash_info.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016143

/*
 * Hash Info: Hash algorithms information
 *
 * Copyright (c) 2013 Dmitry Kasatkin <d.kasatkin@samsung.com>
 */

#[repr(C)]
#[allow(non_camel_case_types)]
pub enum hash_algo {
    HASH_ALGO_MD4 = 0,
    HASH_ALGO_MD5 = 1,
    HASH_ALGO_SHA1 = 2,
    HASH_ALGO_RIPE_MD_160 = 3,
    HASH_ALGO_SHA256 = 4,
    HASH_ALGO_SHA384 = 5,
    HASH_ALGO_SHA512 = 6,
    HASH_ALGO_SHA224 = 7,
    HASH_ALGO_RIPE_MD_128 = 8,
    HASH_ALGO_RIPE_MD_256 = 9,
    HASH_ALGO_RIPE_MD_320 = 10,
    HASH_ALGO_WP_256 = 11,
    HASH_ALGO_WP_384 = 12,
    HASH_ALGO_WP_512 = 13,
    HASH_ALGO_TGR_128 = 14,
    HASH_ALGO_TGR_160 = 15,
    HASH_ALGO_TGR_192 = 16,
    HASH_ALGO_SM3_256 = 17,
    HASH_ALGO_STREEBOG_256 = 18,
    HASH_ALGO_STREEBOG_512 = 19,
    HASH_ALGO_SHA3_256 = 20,
    HASH_ALGO_SHA3_384 = 21,
    HASH_ALGO_SHA3_512 = 22,
    HASH_ALGO__LAST = 23,
}
