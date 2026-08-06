// SPDX-License-Identifier: GPL-2.0
//! linux-source: include/linux/rational.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S014788

/*
 * rational fractions
 *
 * Copyright (C) 2009 emlix GmbH, Oskar Schirmer <oskar@scara.com>
 *
 * helper functions when coping with rational numbers,
 * e.g. when calculating optimum numerator/denominator pairs for
 * pll configuration taking into account restricted register size
 */

use core::ffi::c_ulong;

unsafe extern "C" {
    /// Finds the best rational approximation within the supplied bounds.
    ///
    /// `best_numerator` and `best_denominator` must designate writable
    /// `unsigned long` storage, as required by the C declaration.
    pub fn rational_best_approximation(
        given_numerator: c_ulong,
        given_denominator: c_ulong,
        max_numerator: c_ulong,
        max_denominator: c_ulong,
        best_numerator: *mut c_ulong,
        best_denominator: *mut c_ulong,
    );
}
