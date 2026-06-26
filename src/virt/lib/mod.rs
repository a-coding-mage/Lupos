//! linux-parity: complete
//! linux-source: vendor/linux/virt/lib
//! test-origin: linux:vendor/linux/virt/lib
//! Generic virtualization helper libraries.

pub mod irqbypass;

pub const VIRT_LIB_MODULES: [&str; 1] = ["irqbypass"];
pub const IRQ_BYPASS_KCONFIG_SYMBOL: &str = "IRQ_BYPASS_MANAGER";
pub const IRQ_BYPASS_MAKEFILE_OBJECT: &str = "irqbypass.o";

#[cfg(test)]
mod tests {
    use super::*;

    const KCONFIG: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/virt/lib/Kconfig"
    ));
    const MAKEFILE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/virt/lib/Makefile"
    ));
    const IRQBYPASS_C: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/vendor/linux/virt/lib/irqbypass.c"
    ));

    #[test]
    fn virt_lib_wrapper_matches_linux_source_set() {
        assert_eq!(VIRT_LIB_MODULES, ["irqbypass"]);
        assert_eq!(IRQ_BYPASS_KCONFIG_SYMBOL, "IRQ_BYPASS_MANAGER");
        assert_eq!(IRQ_BYPASS_MAKEFILE_OBJECT, "irqbypass.o");
        assert!(KCONFIG.contains("config IRQ_BYPASS_MANAGER"));
        assert!(KCONFIG.contains("tristate"));
        assert!(MAKEFILE.contains("obj-$(CONFIG_IRQ_BYPASS_MANAGER) += irqbypass.o"));
    }

    #[test]
    fn virt_lib_wrapper_reexports_irqbypass_contract() {
        assert!(IRQBYPASS_C.contains("MODULE_DESCRIPTION(\"IRQ bypass manager utility module\");"));
        assert!(IRQBYPASS_C.contains("static DEFINE_XARRAY(producers);"));
        assert!(IRQBYPASS_C.contains("static DEFINE_XARRAY(consumers);"));

        let mut manager = irqbypass::IrqBypassManager::new();
        let producer = manager
            .register_producer(irqbypass::IrqBypassProducer::new(), 0x10, 32)
            .unwrap();
        let consumer = manager
            .register_consumer(irqbypass::IrqBypassConsumer::new(), 0x10)
            .unwrap();

        assert_eq!(manager.producer(producer).unwrap().consumer, Some(consumer));
        assert_eq!(manager.consumer(consumer).unwrap().producer, Some(producer));
    }
}
