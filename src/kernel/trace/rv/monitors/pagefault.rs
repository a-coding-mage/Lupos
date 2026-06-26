//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/rv/monitors/pagefault/pagefault.c
//! test-origin: linux:vendor/linux/kernel/trace/rv/monitors/pagefault/pagefault.c
//! RV monitor: real-time tasks must not raise page faults.

pub const MONITOR_NAME: &str = "pagefault";
pub const MONITOR_DESCRIPTION: &str = "Monitor that RT tasks do not raise page faults";
pub const MODULE_AUTHOR: &str = "Nam Cao <namcao@linutronix.de>";
pub const MODULE_LICENSE: &str = "GPL";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LtlAtom {
    Pagefault,
    Rt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PagefaultAtoms {
    pub pagefault: bool,
    pub rt: bool,
}

impl PagefaultAtoms {
    pub const fn new(rt: bool) -> Self {
        Self {
            pagefault: false,
            rt,
        }
    }

    pub const fn with_pagefault(mut self) -> Self {
        self.pagefault = true;
        self
    }
}

pub const fn ltl_rule_holds(atoms: PagefaultAtoms) -> bool {
    !atoms.rt || !atoms.pagefault
}

pub const fn pagefault_allowed(rt_task: bool, pagefault: bool) -> bool {
    ltl_rule_holds(PagefaultAtoms {
        rt: rt_task,
        pagefault,
    })
}

pub fn observe(rt_task: bool, page_faulted: bool) -> bool {
    pagefault_allowed(rt_task, page_faulted)
}

pub fn violated(rt_task: bool, page_faulted: bool) -> bool {
    !observe(rt_task, page_faulted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagefault_ltl_rule_matches_linux_header_and_trace_hooks() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/pagefault/pagefault.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/rv/monitors/pagefault/pagefault.h"
        ));
        assert!(source.contains("#define MODULE_NAME \"pagefault\""));
        assert!(source.contains("ltl_atom_set(mon, LTL_RT, rt_or_dl_task(task));"));
        assert!(source.contains("ltl_atom_pulse(current, LTL_PAGEFAULT, true);"));
        assert!(source.contains(
            "rv_attach_trace_probe(\"rtapp_pagefault\", page_fault_kernel, handle_page_fault);"
        ));
        assert!(source.contains(
            "rv_attach_trace_probe(\"rtapp_pagefault\", page_fault_user, handle_page_fault);"
        ));
        assert!(source.contains("rv_register_monitor(&rv_pagefault, &rv_rtapp);"));
        assert!(source.contains(MODULE_AUTHOR));
        assert!(header.contains("LTL_PAGEFAULT"));
        assert!(header.contains("LTL_RT"));
        assert!(header.contains("\"pa\""));
        assert!(header.contains("\"rt\""));
        assert!(header.contains("bool val4 = val1 || val3;"));

        assert!(pagefault_allowed(false, true));
        assert!(pagefault_allowed(true, false));
        assert!(!pagefault_allowed(true, true));
        assert!(violated(true, true));
        assert!(ltl_rule_holds(PagefaultAtoms::new(true)));
        assert!(!ltl_rule_holds(PagefaultAtoms::new(true).with_pagefault()));
    }
}
