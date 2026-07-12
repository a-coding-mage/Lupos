//! linux-parity: partial
//! linux-source: vendor/linux/kernel/rcu
//! RCU — Read-Copy-Update (M34).
//!
//! Will be filled in by the M34 step.  Module declared early so M33's
//! locking primitives can reference `RcuHead` once the wiring lands.

pub mod rcuscale;
pub mod rcutorture;
pub mod refscale;
pub mod segcblist;
pub mod srcu;
pub mod srcutree;
pub mod sync;
pub mod tasks;
pub mod tiny;
pub mod tree;
pub mod types;
pub mod update;

pub use srcu::{SrcuStruct, srcu_read_lock, srcu_read_unlock, synchronize_srcu};
pub use tasks::{call_rcu_tasks, synchronize_rcu_tasks, tasks_rcu_qs};
pub use tree::{
    call_rcu, rcu_barrier, rcu_check_callbacks, rcu_init, rcu_qs, rcu_read_lock, rcu_read_unlock,
    synchronize_rcu,
};
pub use types::{RcuHead, rcu_head_init};

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

/// Register the out-of-line PREEMPT_RCU entry points referenced by modules
/// built with the vendor configuration.
pub fn register_module_exports() {
    export_symbol_once("__rcu_read_lock", linux___rcu_read_lock as usize, true);
    export_symbol_once("__rcu_read_unlock", linux___rcu_read_unlock as usize, true);
}

/// `__rcu_read_lock()` — `vendor/linux/kernel/rcu/tree_plugin.h:412`.
#[unsafe(export_name = "__rcu_read_lock")]
pub extern "C" fn linux___rcu_read_lock() {
    tree::rcu_read_lock();
}

/// `__rcu_read_unlock()` — `vendor/linux/kernel/rcu/tree_plugin.h:430`.
#[unsafe(export_name = "__rcu_read_unlock")]
pub extern "C" fn linux___rcu_read_unlock() {
    tree::rcu_read_unlock();
}
