//! linux-parity: complete
//! linux-source: vendor/linux/mm/init-mm.c
//! test-origin: linux:vendor/linux/mm/init-mm.c
//! Initial kernel mm_struct field initialization.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitMm {
    pub mm_users: u32,
    pub mm_count: u32,
    pub start_code: usize,
    pub end_code: usize,
    pub end_data: usize,
    pub brk: usize,
    pub has_user_ns: bool,
    pub has_mmap_lock: bool,
    pub has_page_table_lock: bool,
}

pub const INIT_MM: InitMm = InitMm {
    mm_users: 2,
    mm_count: 1,
    start_code: 0,
    end_code: 0,
    end_data: 0,
    brk: 0,
    has_user_ns: true,
    has_mmap_lock: true,
    has_page_table_lock: true,
};

pub const fn setup_initial_init_mm(
    start_code: usize,
    end_code: usize,
    end_data: usize,
    brk: usize,
) -> InitMm {
    InitMm {
        start_code,
        end_code,
        end_data,
        brk,
        ..INIT_MM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_mm_defaults_and_setup_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/init-mm.c"
        ));
        assert!(source.contains("const struct vm_operations_struct vma_dummy_vm_ops;"));
        assert!(source.contains("struct mm_struct init_mm = {"));
        assert!(source.contains(".mm_users\t= ATOMIC_INIT(2),"));
        assert!(source.contains(".mm_count\t= ATOMIC_INIT(1),"));
        assert!(source.contains(".user_ns\t= &init_user_ns,"));
        assert!(source.contains("void setup_initial_init_mm"));
        assert!(source.contains("init_mm.start_code = (unsigned long)start_code;"));
        assert!(source.contains("init_mm.brk = (unsigned long)brk;"));

        assert_eq!(INIT_MM.mm_users, 2);
        assert_eq!(INIT_MM.mm_count, 1);
        let mm = setup_initial_init_mm(1, 2, 3, 4);
        assert_eq!(mm.start_code, 1);
        assert_eq!(mm.end_code, 2);
        assert_eq!(mm.end_data, 3);
        assert_eq!(mm.brk, 4);
        assert!(mm.has_mmap_lock);
    }
}
