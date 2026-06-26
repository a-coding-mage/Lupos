//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/bugs_64.c
//! test-origin: linux:vendor/linux/arch/x86/um/bugs_64.c
//! UML x86-64 bug check hooks.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UmlPtRegs;

pub fn arch_check_bugs() {}

pub fn arch_examine_signal(_sig: i32, _regs: &mut UmlPtRegs) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hooks_are_linux_noops() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/bugs_64.c"
        ));
        assert!(source.contains("void arch_check_bugs(void)"));
        assert!(source.contains("void arch_examine_signal(int sig, struct uml_pt_regs *regs)"));

        let mut regs = UmlPtRegs;
        arch_check_bugs();
        arch_examine_signal(11, &mut regs);
        assert_eq!(regs, UmlPtRegs);
    }
}
