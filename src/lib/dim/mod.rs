//! linux-parity: complete
//! linux-source: vendor/linux/lib/dim
//! Dynamic Interrupt Moderation library helpers.

pub mod dim;
pub mod rdma_dim;

pub fn register_module_exports() {
    dim::register_module_exports();
    rdma_dim::register_module_exports();
}
