// SPDX-License-Identifier: BSD-3-Clause OR GPL-2.0
//! linux-source: include/acpi/platform/acgccex.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S012491

// Copyright (C) 2000 - 2026, Intel Corp.

// The source header is a GCC-preprocessor-only workaround: when a macro named
// `strchr` exists, it undefines that macro before ACPICA code is parsed. Rust
// has no C preprocessor macro namespace and this module declares no `strchr`
// binding, so the corresponding Rust translation has no exported item.
