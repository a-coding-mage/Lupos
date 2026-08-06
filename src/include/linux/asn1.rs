// SPDX-License-Identifier: GPL-2.0-or-later
//! linux-source: include/linux/asn1.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S013468

// ASN.1 BER/DER/CER encoding definitions
//
// Copyright (C) 2012 Red Hat, Inc. All Rights Reserved.
// Written by David Howells (dhowells@redhat.com)

/*
 * C enumeration constants have type `int` and are used by the selected
 * callers in integer tag-byte expressions.  The named C enum types have no
 * storage or function-parameter use in this header's selected kernel context,
 * so retain both their C `int` representation and the enumerators' ordinary
 * integer-expression semantics.
 */

/* Class */
pub type asn1_class = core::ffi::c_int;

pub const ASN1_UNIV: asn1_class = 0; /* Universal */
pub const ASN1_APPL: asn1_class = 1; /* Application */
pub const ASN1_CONT: asn1_class = 2; /* Context */
pub const ASN1_PRIV: asn1_class = 3; /* Private */

pub const ASN1_CLASS_BITS: core::ffi::c_int = 0xc0;

pub type asn1_method = core::ffi::c_int;

pub const ASN1_PRIM: asn1_method = 0; /* Primitive */
pub const ASN1_CONS: asn1_method = 1; /* Constructed */

pub const ASN1_CONS_BIT: core::ffi::c_int = 0x20;

/* Tag */
pub type asn1_tag = core::ffi::c_int;

pub const ASN1_EOC: asn1_tag = 0; /* End Of Contents or N/A */
pub const ASN1_BOOL: asn1_tag = 1; /* Boolean */
pub const ASN1_INT: asn1_tag = 2; /* Integer */
pub const ASN1_BTS: asn1_tag = 3; /* Bit String */
pub const ASN1_OTS: asn1_tag = 4; /* Octet String */
pub const ASN1_NULL: asn1_tag = 5; /* Null */
pub const ASN1_OID: asn1_tag = 6; /* Object Identifier */
pub const ASN1_ODE: asn1_tag = 7; /* Object Description */
pub const ASN1_EXT: asn1_tag = 8; /* External */
pub const ASN1_REAL: asn1_tag = 9; /* Real float */
pub const ASN1_ENUM: asn1_tag = 10; /* Enumerated */
pub const ASN1_EPDV: asn1_tag = 11; /* Embedded PDV */
pub const ASN1_UTF8STR: asn1_tag = 12; /* UTF8 String */
pub const ASN1_RELOID: asn1_tag = 13; /* Relative OID */
/* 14 - Reserved */
/* 15 - Reserved */
pub const ASN1_SEQ: asn1_tag = 16; /* Sequence and Sequence of */
pub const ASN1_SET: asn1_tag = 17; /* Set and Set of */
pub const ASN1_NUMSTR: asn1_tag = 18; /* Numerical String */
pub const ASN1_PRNSTR: asn1_tag = 19; /* Printable String */
pub const ASN1_TEXSTR: asn1_tag = 20; /* T61 String / Teletext String */
pub const ASN1_VIDSTR: asn1_tag = 21; /* Videotex String */
pub const ASN1_IA5STR: asn1_tag = 22; /* IA5 String */
pub const ASN1_UNITIM: asn1_tag = 23; /* Universal Time */
pub const ASN1_GENTIM: asn1_tag = 24; /* General Time */
pub const ASN1_GRASTR: asn1_tag = 25; /* Graphic String */
pub const ASN1_VISSTR: asn1_tag = 26; /* Visible String */
pub const ASN1_GENSTR: asn1_tag = 27; /* General String */
pub const ASN1_UNISTR: asn1_tag = 28; /* Universal String */
pub const ASN1_CHRSTR: asn1_tag = 29; /* Character String */
pub const ASN1_BMPSTR: asn1_tag = 30; /* BMP String */
pub const ASN1_LONG_TAG: asn1_tag = 31; /* Long form tag */

pub const ASN1_INDEFINITE_LENGTH: core::ffi::c_int = 0x80;
