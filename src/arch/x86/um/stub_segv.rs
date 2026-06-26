//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/stub_segv.c
//! test-origin: linux:vendor/linux/arch/x86/um/stub_segv.c
//! UML syscall-stub SIGSEGV handler.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FaultInfo {
    pub address: usize,
    pub trap_no: i32,
    pub error_code: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UContext {
    pub mcontext_fault: FaultInfo,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StubData {
    pub fault: FaultInfo,
    pub trapped: bool,
}

pub fn stub_segv_handler(_sig: i32, _info_present: bool, context: &UContext, data: &mut StubData) {
    data.fault = context.mcontext_fault;
    data.trapped = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segv_handler_copies_faultinfo_and_traps() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/stub_segv.c"
        ));
        assert!(source.contains("GET_FAULTINFO_FROM_MC(*f, &uc->uc_mcontext);"));
        assert!(source.contains("trap_myself();"));

        let context = UContext {
            mcontext_fault: FaultInfo {
                address: 0xdead,
                trap_no: 14,
                error_code: 4,
            },
        };
        let mut data = StubData::default();
        stub_segv_handler(11, true, &context, &mut data);
        assert_eq!(data.fault, context.mcontext_fault);
        assert!(data.trapped);
    }
}
