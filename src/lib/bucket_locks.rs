//! linux-parity: complete
//! linux-source: vendor/linux/lib/bucket_locks.c
//! test-origin: linux:vendor/linux/lib/bucket_locks.c
//! Bucket spinlock allocation sizing and init metadata.

use crate::include::uapi::errno::ENOMEM;

pub const PROVE_LOCKING_NR_PCPUS: u32 = 2;
pub const MAX_BUCKET_LOCK_CPUS: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BucketSpinlockPlan {
    pub lock_count: u32,
    pub locks_mask: u32,
    pub nr_pcpus: u32,
}

pub const fn bucket_spinlock_cpu_count(num_possible_cpus: u32, prove_locking: bool) -> u32 {
    if prove_locking {
        PROVE_LOCKING_NR_PCPUS
    } else {
        num_possible_cpus
    }
}

pub const fn bucket_spinlock_count(
    max_size: u32,
    cpu_mult: u32,
    num_possible_cpus: u32,
    prove_locking: bool,
) -> u32 {
    if cpu_mult == 0 {
        return max_size;
    }

    let mut nr_pcpus = bucket_spinlock_cpu_count(num_possible_cpus, prove_locking);
    if nr_pcpus > MAX_BUCKET_LOCK_CPUS {
        nr_pcpus = MAX_BUCKET_LOCK_CPUS;
    }

    let requested = nr_pcpus.saturating_mul(cpu_mult);
    if requested < max_size {
        requested
    } else {
        max_size
    }
}

pub const fn bucket_spinlock_plan(
    max_size: u32,
    cpu_mult: u32,
    num_possible_cpus: u32,
    prove_locking: bool,
) -> BucketSpinlockPlan {
    let lock_count = bucket_spinlock_count(max_size, cpu_mult, num_possible_cpus, prove_locking);
    BucketSpinlockPlan {
        lock_count,
        locks_mask: lock_count.wrapping_sub(1),
        nr_pcpus: bucket_spinlock_cpu_count(num_possible_cpus, prove_locking),
    }
}

pub const fn alloc_bucket_spinlocks_model(
    allocation_succeeds: bool,
    max_size: u32,
    cpu_mult: u32,
    num_possible_cpus: u32,
    prove_locking: bool,
) -> Result<BucketSpinlockPlan, i32> {
    let plan = bucket_spinlock_plan(max_size, cpu_mult, num_possible_cpus, prove_locking);
    if plan.lock_count != 0 && !allocation_succeeds {
        Err(-ENOMEM)
    } else {
        Ok(plan)
    }
}

pub const fn free_bucket_spinlocks_model(locks_were_allocated: bool) -> bool {
    !locks_were_allocated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_lock_sizing_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/bucket_locks.c"
        ));
        assert!(source.contains("unsigned int nr_pcpus = 2;"));
        assert!(source.contains("nr_pcpus = num_possible_cpus();"));
        assert!(source.contains("nr_pcpus = min_t(unsigned int, nr_pcpus, 64UL);"));
        assert!(source.contains("size = min_t(unsigned int, nr_pcpus * cpu_mult, max_size);"));
        assert!(source.contains("*locks_mask = size - 1;"));
        assert!(source.contains("EXPORT_SYMBOL(__alloc_bucket_spinlocks);"));
        assert!(source.contains("EXPORT_SYMBOL(free_bucket_spinlocks);"));

        assert_eq!(bucket_spinlock_count(1024, 4, 256, false), 256);
        assert_eq!(bucket_spinlock_count(1024, 4, 8, false), 32);
        assert_eq!(bucket_spinlock_count(1024, 0, 8, false), 1024);
        assert_eq!(bucket_spinlock_count(1024, 4, 256, true), 8);

        let plan = alloc_bucket_spinlocks_model(true, 1024, 4, 8, false).unwrap();
        assert_eq!(
            plan,
            BucketSpinlockPlan {
                lock_count: 32,
                locks_mask: 31,
                nr_pcpus: 8,
            }
        );
        assert_eq!(
            alloc_bucket_spinlocks_model(false, 1024, 4, 8, false),
            Err(-ENOMEM)
        );
        assert!(free_bucket_spinlocks_model(false));
    }
}
