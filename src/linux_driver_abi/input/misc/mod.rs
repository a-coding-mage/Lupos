//! linux-parity: partial
//! linux-source: vendor/linux/drivers/input/misc
//! test-origin: linux:vendor/linux/drivers/input/misc
//! Miscellaneous input drivers.
//!
//! Mirrors `drivers/input/misc`. Currently provides the PC speaker beeper
//! (`pcspkr.c`) that backs the virtual-console bell.

pub mod pcspkr;
