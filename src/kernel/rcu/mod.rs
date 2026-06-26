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
