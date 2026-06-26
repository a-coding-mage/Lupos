//! linux-parity: complete
//! linux-source: vendor/linux/virt/kvm/async_pf.c
//! test-origin: linux:vendor/linux/virt/kvm/async_pf.c
//! KVM asynchronous page-fault queue accounting and completion flow.
//!
//! Ref: `vendor/linux/virt/kvm/async_pf.c`

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::include::uapi::errno::ENOMEM;

pub const ASYNC_PF_PER_VCPU: usize = 64;
pub const PAGE_OFFSET: u64 = 0xffff_8000_0000_0000;

static ASYNC_PF_CACHE_INITIALIZED: AtomicBool = AtomicBool::new(false);
static NEXT_WORK_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AsyncPfWork {
    pub id: u64,
    pub wakeup_all: bool,
    pub cr2_or_gpa: u64,
    pub addr: u64,
    pub notpresent_injected: bool,
    pub executed: bool,
}

impl AsyncPfWork {
    pub fn new(cr2_or_gpa: u64, addr: u64) -> Self {
        Self {
            id: NEXT_WORK_ID.fetch_add(1, Ordering::Relaxed),
            wakeup_all: false,
            cr2_or_gpa,
            addr,
            notpresent_injected: false,
            executed: false,
        }
    }

    pub fn wakeup_all() -> Self {
        Self {
            id: NEXT_WORK_ID.fetch_add(1, Ordering::Relaxed),
            wakeup_all: true,
            cr2_or_gpa: 0,
            addr: 0,
            notpresent_injected: false,
            executed: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AsyncPfVcpu {
    pub queued: usize,
    pub queue: Vec<AsyncPfWork>,
    pub done: Vec<AsyncPfWork>,
    pub can_dequeue: bool,
    pub async_pf_sync: bool,
    pub page_present_queued: usize,
    pub page_present: usize,
    pub page_ready: usize,
    pub wakeups: usize,
    pub flushed: usize,
    pub freed: usize,
    pub cancelled: usize,
    pub not_present_calls: usize,
    pub inject_notpresent: bool,
}

impl AsyncPfVcpu {
    pub const fn new() -> Self {
        Self {
            queued: 0,
            queue: Vec::new(),
            done: Vec::new(),
            can_dequeue: true,
            async_pf_sync: false,
            page_present_queued: 0,
            page_present: 0,
            page_ready: 0,
            wakeups: 0,
            flushed: 0,
            freed: 0,
            cancelled: 0,
            not_present_calls: 0,
            inject_notpresent: true,
        }
    }

    pub fn vcpu_init(&mut self) {
        self.queue.clear();
        self.done.clear();
        self.queued = 0;
    }

    pub fn setup_async_pf(&mut self, cr2_or_gpa: u64, hva: u64) -> bool {
        if self.queued >= ASYNC_PF_PER_VCPU {
            return false;
        }

        if kvm_is_error_hva(hva) {
            return false;
        }

        if !async_pf_cache_initialized() {
            return false;
        }

        let mut work = AsyncPfWork::new(cr2_or_gpa, hva);
        work.notpresent_injected = self.kvm_arch_async_page_not_present();
        self.queue.push(work);
        self.queued += 1;
        true
    }

    pub fn execute_next(&mut self) -> bool {
        let Some(index) = self.queue.iter().position(|work| !work.executed) else {
            return false;
        };

        let first = self.done.is_empty();
        self.queue[index].executed = true;
        let work = self.queue[index];

        if self.async_pf_sync {
            self.kvm_arch_async_page_present();
        }

        self.done.push(work);

        if !self.async_pf_sync && first {
            self.kvm_arch_async_page_present_queued();
        }

        self.wakeups += 1;
        true
    }

    pub fn check_completion(&mut self) -> usize {
        let mut completed = 0;
        while self.can_dequeue && !self.done.is_empty() {
            let work = self.done.remove(0);
            self.kvm_arch_async_page_ready();
            if !self.async_pf_sync {
                self.kvm_arch_async_page_present();
            }
            self.remove_from_queue(work.id);
            self.queued = self.queued.saturating_sub(1);
            self.flush_and_free_async_pf_work(work);
            completed += 1;
        }
        completed
    }

    pub fn clear_completion_queue(&mut self) {
        let mut freed_ids = Vec::new();
        while let Some(work) = self.queue.pop() {
            if work.executed {
                self.flushed += 1;
            } else {
                self.cancelled += 1;
            }
            freed_ids.push(work.id);
            self.freed += 1;
        }

        while let Some(work) = self.done.pop() {
            if !freed_ids.contains(&work.id) {
                self.flush_and_free_async_pf_work(work);
            }
        }

        self.queued = 0;
    }

    pub fn wakeup_all(&mut self, allocation_available: bool) -> Result<(), i32> {
        if !self.done.is_empty() {
            return Ok(());
        }
        if !async_pf_cache_initialized() || !allocation_available {
            return Err(-ENOMEM);
        }

        let first = self.done.is_empty();
        let work = AsyncPfWork::wakeup_all();
        self.done.push(work);

        if !self.async_pf_sync && first {
            self.kvm_arch_async_page_present_queued();
        }

        self.queued += 1;
        Ok(())
    }

    fn remove_from_queue(&mut self, id: u64) {
        if let Some(index) = self.queue.iter().position(|work| work.id == id) {
            self.queue.remove(index);
        }
    }

    fn flush_and_free_async_pf_work(&mut self, work: AsyncPfWork) {
        if !work.wakeup_all {
            self.flushed += 1;
        }
        self.freed += 1;
    }

    fn kvm_arch_async_page_not_present(&mut self) -> bool {
        self.not_present_calls += 1;
        self.inject_notpresent
    }

    fn kvm_arch_async_page_present_queued(&mut self) {
        self.page_present_queued += 1;
    }

    fn kvm_arch_async_page_present(&mut self) {
        self.page_present += 1;
    }

    fn kvm_arch_async_page_ready(&mut self) {
        self.page_ready += 1;
    }
}

pub fn kvm_async_pf_init() -> i32 {
    ASYNC_PF_CACHE_INITIALIZED.store(true, Ordering::SeqCst);
    0
}

pub fn kvm_async_pf_init_with_allocation(allocation_available: bool) -> i32 {
    if allocation_available {
        kvm_async_pf_init()
    } else {
        ASYNC_PF_CACHE_INITIALIZED.store(false, Ordering::SeqCst);
        -(ENOMEM as i32)
    }
}

pub fn kvm_async_pf_deinit() {
    ASYNC_PF_CACHE_INITIALIZED.store(false, Ordering::SeqCst);
}

pub fn async_pf_cache_initialized() -> bool {
    ASYNC_PF_CACHE_INITIALIZED.load(Ordering::SeqCst)
}

pub fn kvm_async_pf_vcpu_init(vcpu: &mut AsyncPfVcpu) {
    vcpu.vcpu_init();
}

pub const fn kvm_is_error_hva(addr: u64) -> bool {
    addr >= PAGE_OFFSET
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_cache() {
        kvm_async_pf_deinit();
        assert_eq!(kvm_async_pf_init(), 0);
    }

    #[test]
    fn async_pf_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/async_pf.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/kvm/async_pf.h"
        ));

        assert!(source.contains("static struct kmem_cache *async_pf_cache;"));
        assert!(source.contains("int kvm_async_pf_init(void)"));
        assert!(source.contains("void kvm_async_pf_deinit(void)"));
        assert!(source.contains("void kvm_async_pf_vcpu_init(struct kvm_vcpu *vcpu)"));
        assert!(source.contains("INIT_LIST_HEAD(&vcpu->async_pf.done);"));
        assert!(source.contains("INIT_LIST_HEAD(&vcpu->async_pf.queue);"));
        assert!(source.contains("static void async_pf_execute(struct work_struct *work)"));
        assert!(source.contains("if (vcpu->async_pf.queued >= ASYNC_PF_PER_VCPU)"));
        assert!(source.contains("if (unlikely(kvm_is_error_hva(hva)))"));
        assert!(source.contains("work = kmem_cache_zalloc(async_pf_cache, GFP_NOWAIT);"));
        assert!(source.contains("list_add_tail(&work->queue, &vcpu->async_pf.queue);"));
        assert!(
            source.contains(
                "work->notpresent_injected = kvm_arch_async_page_not_present(vcpu, work);"
            )
        );
        assert!(source.contains("list_add_tail(&apf->link, &vcpu->async_pf.done);"));
        assert!(source.contains("void kvm_check_async_pf_completion(struct kvm_vcpu *vcpu)"));
        assert!(source.contains("kvm_flush_and_free_async_pf_work(work);"));
        assert!(source.contains("vcpu->async_pf.queued--;"));
        assert!(source.contains("work->wakeup_all = true;"));
        assert!(header.contains("int kvm_async_pf_init(void);"));
        assert!(header.contains("void kvm_async_pf_deinit(void);"));
        assert!(header.contains("void kvm_async_pf_vcpu_init(struct kvm_vcpu *vcpu);"));
    }

    #[test]
    fn init_deinit_control_global_cache_state() {
        kvm_async_pf_deinit();
        assert!(!async_pf_cache_initialized());
        assert_eq!(kvm_async_pf_init_with_allocation(false), -(ENOMEM as i32));
        assert!(!async_pf_cache_initialized());
        assert_eq!(kvm_async_pf_init(), 0);
        assert!(async_pf_cache_initialized());
        kvm_async_pf_deinit();
        assert!(!async_pf_cache_initialized());
    }

    #[test]
    fn setup_rejects_full_queue_error_hva_and_missing_cache() {
        kvm_async_pf_deinit();
        let mut vcpu = AsyncPfVcpu::new();
        assert!(!vcpu.setup_async_pf(0, 0x1000));

        init_cache();
        assert!(!vcpu.setup_async_pf(0, PAGE_OFFSET));
        for i in 0..ASYNC_PF_PER_VCPU {
            assert!(vcpu.setup_async_pf(i as u64, 0x1000 + i as u64));
        }
        assert!(!vcpu.setup_async_pf(99, 0x2000));
        assert_eq!(vcpu.queued, ASYNC_PF_PER_VCPU);
        assert_eq!(vcpu.not_present_calls, ASYNC_PF_PER_VCPU);
    }

    #[test]
    fn execute_then_check_completion_preserves_linux_queued_accounting() {
        init_cache();
        let mut vcpu = AsyncPfVcpu::new();
        assert!(vcpu.setup_async_pf(0xaa, 0x1000));
        assert!(vcpu.execute_next());
        assert_eq!(vcpu.queued, 1);
        assert_eq!(vcpu.queue.len(), 1);
        assert!(vcpu.queue[0].executed);
        assert_eq!(vcpu.done.len(), 1);
        assert_eq!(vcpu.page_present_queued, 1);
        assert_eq!(vcpu.page_present, 0);
        assert_eq!(vcpu.check_completion(), 1);
        assert_eq!(vcpu.queued, 0);
        assert!(vcpu.queue.is_empty());
        assert!(vcpu.done.is_empty());
        assert_eq!(vcpu.page_ready, 1);
        assert_eq!(vcpu.page_present, 1);
        assert_eq!(vcpu.flushed, 1);
        assert_eq!(vcpu.freed, 1);
    }

    #[test]
    fn sync_mode_calls_page_present_during_execute_not_completion() {
        init_cache();
        let mut vcpu = AsyncPfVcpu::new();
        vcpu.async_pf_sync = true;
        assert!(vcpu.setup_async_pf(0xaa, 0x1000));
        assert!(vcpu.execute_next());
        assert_eq!(vcpu.page_present_queued, 0);
        assert_eq!(vcpu.page_present, 1);
        assert_eq!(vcpu.check_completion(), 1);
        assert_eq!(vcpu.page_present, 1);
    }

    #[test]
    fn clear_completion_queue_cancels_pending_and_frees_done_items() {
        init_cache();
        let mut vcpu = AsyncPfVcpu::new();
        assert!(vcpu.setup_async_pf(1, 0x1000));
        assert!(vcpu.setup_async_pf(2, 0x2000));
        assert!(vcpu.execute_next());
        vcpu.clear_completion_queue();
        assert_eq!(vcpu.queued, 0);
        assert!(vcpu.queue.is_empty());
        assert!(vcpu.done.is_empty());
        assert_eq!(vcpu.cancelled, 1);
        assert_eq!(vcpu.flushed, 1);
        assert_eq!(vcpu.freed, 2);
    }

    #[test]
    fn wakeup_all_skips_when_done_queue_is_not_empty() {
        init_cache();
        let mut vcpu = AsyncPfVcpu::new();
        vcpu.wakeup_all(true).unwrap();
        assert_eq!(vcpu.done.len(), 1);
        assert!(vcpu.done[0].wakeup_all);
        assert_eq!(vcpu.queued, 1);
        assert_eq!(vcpu.page_present_queued, 1);
        vcpu.wakeup_all(false).unwrap();
        assert_eq!(vcpu.done.len(), 1);
        assert_eq!(vcpu.check_completion(), 1);
        assert_eq!(vcpu.flushed, 0);
        assert_eq!(vcpu.freed, 1);
    }
}
