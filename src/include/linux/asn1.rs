// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/linux/asn1.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013468

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum asn1_class {
    ASN1_UNIV = 0,
    ASN1_APPL = 1,
    ASN1_CONT = 2,
    ASN1_PRIV = 3,
}

pub use asn1_class::{ASN1_APPL, ASN1_CONT, ASN1_PRIV, ASN1_UNIV};

pub const ASN1_CLASS_BITS: i32 = 0xc0;

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum asn1_method {
    ASN1_PRIM = 0,
    ASN1_CONS = 1,
}

pub use asn1_method::{ASN1_CONS, ASN1_PRIM};

pub const ASN1_CONS_BIT: i32 = 0x20;

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum asn1_tag {
    ASN1_EOC = 0,
    ASN1_BOOL = 1,
    ASN1_INT = 2,
    ASN1_BTS = 3,
    ASN1_OTS = 4,
    ASN1_NULL = 5,
    ASN1_OID = 6,
    ASN1_ODE = 7,
    ASN1_EXT = 8,
    ASN1_REAL = 9,
    ASN1_ENUM = 10,
    ASN1_EPDV = 11,
    ASN1_UTF8STR = 12,
    ASN1_RELOID = 13,
    ASN1_SEQ = 16,
    ASN1_SET = 17,
    ASN1_NUMSTR = 18,
    ASN1_PRNSTR = 19,
    ASN1_TEXSTR = 20,
    ASN1_VIDSTR = 21,
    ASN1_IA5STR = 22,
    ASN1_UNITIM = 23,
    ASN1_GENTIM = 24,
    ASN1_GRASTR = 25,
    ASN1_VISSTR = 26,
    ASN1_GENSTR = 27,
    ASN1_UNISTR = 28,
    ASN1_CHRSTR = 29,
    ASN1_BMPSTR = 30,
    ASN1_LONG_TAG = 31,
}

pub use asn1_tag::{
    ASN1_BOOL, ASN1_BMPSTR, ASN1_BTS, ASN1_CHRSTR, ASN1_EOC, ASN1_ENUM, ASN1_EPDV,
    ASN1_EXT, ASN1_GENSTR, ASN1_GENTIM, ASN1_GRASTR, ASN1_IA5STR, ASN1_INT,
    ASN1_LONG_TAG, ASN1_NULL, ASN1_ODE, ASN1_OID, ASN1_OTS, ASN1_PRNSTR, ASN1_RELOID,
    ASN1_REAL, ASN1_SEQ, ASN1_SET, ASN1_TEXSTR, ASN1_UNISTR, ASN1_UNITIM, ASN1_UTF8STR,
    ASN1_VIDSTR, ASN1_VISSTR,
};

pub const ASN1_INDEFINITE_LENGTH: i32 = 0x80;
