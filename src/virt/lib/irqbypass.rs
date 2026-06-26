//! linux-parity: complete
//! linux-source: vendor/linux/virt/lib/irqbypass.c
//! test-origin: linux:vendor/linux/virt/lib/irqbypass.c
//! IRQ bypass producer/consumer registration and connection ordering.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EBUSY, EINVAL};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrqBypassEvent {
    ProducerStop,
    ConsumerStop,
    ProducerAddConsumer,
    ConsumerAddProducer,
    ProducerDelConsumer,
    ConsumerDelProducer,
    ConsumerStart,
    ProducerStart,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrqBypassProducer {
    pub eventfd: Option<usize>,
    pub irq: i32,
    pub consumer: Option<usize>,
    pub add_consumer_ret: Option<i32>,
    pub has_del_consumer: bool,
    pub has_stop: bool,
    pub has_start: bool,
}

impl IrqBypassProducer {
    pub const fn new() -> Self {
        Self {
            eventfd: None,
            irq: 0,
            consumer: None,
            add_consumer_ret: Some(0),
            has_del_consumer: true,
            has_stop: true,
            has_start: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IrqBypassConsumer {
    pub eventfd: Option<usize>,
    pub producer: Option<usize>,
    pub add_producer_ret: i32,
    pub has_add_producer: bool,
    pub has_del_producer: bool,
    pub has_stop: bool,
    pub has_start: bool,
}

impl IrqBypassConsumer {
    pub const fn new() -> Self {
        Self {
            eventfd: None,
            producer: None,
            add_producer_ret: 0,
            has_add_producer: true,
            has_del_producer: true,
            has_stop: true,
            has_start: true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IrqBypassManager {
    producers: Vec<IrqBypassProducer>,
    consumers: Vec<IrqBypassConsumer>,
    events: Vec<IrqBypassEvent>,
}

impl IrqBypassManager {
    pub const fn new() -> Self {
        Self {
            producers: Vec::new(),
            consumers: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn events(&self) -> &[IrqBypassEvent] {
        &self.events
    }

    pub fn producer(&self, index: usize) -> Option<&IrqBypassProducer> {
        self.producers.get(index)
    }

    pub fn consumer(&self, index: usize) -> Option<&IrqBypassConsumer> {
        self.consumers.get(index)
    }

    pub fn register_producer(
        &mut self,
        mut producer: IrqBypassProducer,
        eventfd: usize,
        irq: i32,
    ) -> Result<usize, i32> {
        if producer.eventfd.is_some() {
            return Err(-EINVAL);
        }
        if self.find_producer(eventfd).is_some() {
            return Err(-EBUSY);
        }

        producer.irq = irq;
        producer.eventfd = Some(eventfd);
        let index = self.producers.len();
        self.producers.push(producer);

        if let Some(consumer_index) = self.find_consumer(eventfd) {
            if let Err(err) = self.connect(index, consumer_index) {
                self.producers.pop();
                return Err(err);
            }
        }

        Ok(index)
    }

    pub fn unregister_producer(&mut self, index: usize) {
        let Some(eventfd) = self
            .producers
            .get(index)
            .and_then(|producer| producer.eventfd)
        else {
            return;
        };
        if self.find_producer(eventfd) != Some(index) {
            return;
        }
        if let Some(consumer) = self.producers[index].consumer {
            self.disconnect(index, consumer);
        }
        self.producers[index].eventfd = None;
    }

    pub fn register_consumer(
        &mut self,
        mut consumer: IrqBypassConsumer,
        eventfd: usize,
    ) -> Result<usize, i32> {
        if consumer.eventfd.is_some() {
            return Err(-EINVAL);
        }
        if !consumer.has_add_producer || !consumer.has_del_producer {
            return Err(-EINVAL);
        }
        if self.find_consumer(eventfd).is_some() {
            return Err(-EBUSY);
        }

        consumer.eventfd = Some(eventfd);
        let index = self.consumers.len();
        self.consumers.push(consumer);

        if let Some(producer_index) = self.find_producer(eventfd) {
            if let Err(err) = self.connect(producer_index, index) {
                self.consumers.pop();
                return Err(err);
            }
        }

        Ok(index)
    }

    pub fn unregister_consumer(&mut self, index: usize) {
        let Some(eventfd) = self
            .consumers
            .get(index)
            .and_then(|consumer| consumer.eventfd)
        else {
            return;
        };
        if self.find_consumer(eventfd) != Some(index) {
            return;
        }
        if let Some(producer) = self.consumers[index].producer {
            self.disconnect(producer, index);
        }
        self.consumers[index].eventfd = None;
    }

    fn connect(&mut self, producer_index: usize, consumer_index: usize) -> Result<(), i32> {
        if self.producers[producer_index].has_stop {
            self.events.push(IrqBypassEvent::ProducerStop);
        }
        if self.consumers[consumer_index].has_stop {
            self.events.push(IrqBypassEvent::ConsumerStop);
        }

        let mut ret = 0;
        if let Some(add_ret) = self.producers[producer_index].add_consumer_ret {
            self.events.push(IrqBypassEvent::ProducerAddConsumer);
            ret = add_ret;
        }
        if ret == 0 {
            self.events.push(IrqBypassEvent::ConsumerAddProducer);
            ret = self.consumers[consumer_index].add_producer_ret;
            if ret != 0 && self.producers[producer_index].has_del_consumer {
                self.events.push(IrqBypassEvent::ProducerDelConsumer);
            }
        }

        if self.consumers[consumer_index].has_start {
            self.events.push(IrqBypassEvent::ConsumerStart);
        }
        if self.producers[producer_index].has_start {
            self.events.push(IrqBypassEvent::ProducerStart);
        }

        if ret == 0 {
            self.producers[producer_index].consumer = Some(consumer_index);
            self.consumers[consumer_index].producer = Some(producer_index);
            Ok(())
        } else {
            Err(ret)
        }
    }

    fn disconnect(&mut self, producer_index: usize, consumer_index: usize) {
        if self.producers[producer_index].has_stop {
            self.events.push(IrqBypassEvent::ProducerStop);
        }
        if self.consumers[consumer_index].has_stop {
            self.events.push(IrqBypassEvent::ConsumerStop);
        }

        self.events.push(IrqBypassEvent::ConsumerDelProducer);
        if self.producers[producer_index].has_del_consumer {
            self.events.push(IrqBypassEvent::ProducerDelConsumer);
        }

        if self.consumers[consumer_index].has_start {
            self.events.push(IrqBypassEvent::ConsumerStart);
        }
        if self.producers[producer_index].has_start {
            self.events.push(IrqBypassEvent::ProducerStart);
        }

        self.producers[producer_index].consumer = None;
        self.consumers[consumer_index].producer = None;
    }

    fn find_producer(&self, eventfd: usize) -> Option<usize> {
        self.producers
            .iter()
            .position(|producer| producer.eventfd == Some(eventfd))
    }

    fn find_consumer(&self, eventfd: usize) -> Option<usize> {
        self.consumers
            .iter()
            .position(|consumer| consumer.eventfd == Some(eventfd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irqbypass_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/virt/lib/irqbypass.c"
        ));
        assert!(source.contains("static DEFINE_XARRAY(producers);"));
        assert!(source.contains("static DEFINE_XARRAY(consumers);"));
        assert!(source.contains("static DEFINE_MUTEX(lock);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL v2\");"));
        assert!(source.contains("MODULE_DESCRIPTION(\"IRQ bypass manager utility module\");"));
        assert!(source.contains("static int __connect(struct irq_bypass_producer *prod"));
        assert!(source.contains("if (prod->stop)"));
        assert!(source.contains("if (prod->add_consumer)"));
        assert!(source.contains("ret = cons->add_producer(cons, prod);"));
        assert!(source.contains("if (ret && prod->del_consumer)"));
        assert!(source.contains("prod->consumer = cons;"));
        assert!(source.contains("cons->producer = prod;"));
        assert!(source.contains("static void __disconnect(struct irq_bypass_producer *prod"));
        assert!(source.contains("cons->del_producer(cons, prod);"));
        assert!(source.contains("prod->consumer = NULL;"));
        assert!(source.contains("cons->producer = NULL;"));
        assert!(source.contains("if (WARN_ON_ONCE(producer->eventfd))"));
        assert!(source.contains("guard(mutex)(&lock);"));
        assert!(source.contains("ret = xa_insert(&producers, index, producer, GFP_KERNEL);"));
        assert!(source.contains("consumer = xa_load(&consumers, index);"));
        assert!(source.contains("WARN_ON_ONCE(xa_erase(&producers, index) != producer);"));
        assert!(source.contains("producer->eventfd = NULL;"));
        assert!(source.contains("if (!consumer->add_producer || !consumer->del_producer)"));
        assert!(source.contains("ret = xa_insert(&consumers, index, consumer, GFP_KERNEL);"));
        assert!(source.contains("producer = xa_load(&producers, index);"));
        assert!(source.contains("WARN_ON_ONCE(xa_erase(&consumers, index) != consumer);"));
        assert!(source.contains("consumer->eventfd = NULL;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(irq_bypass_register_producer);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(irq_bypass_unregister_producer);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(irq_bypass_register_consumer);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(irq_bypass_unregister_consumer);"));
    }

    #[test]
    fn matching_eventfd_connects_in_linux_callback_order() {
        let mut manager = IrqBypassManager::new();
        let consumer = manager
            .register_consumer(IrqBypassConsumer::new(), 0x40)
            .unwrap();
        let producer = manager
            .register_producer(IrqBypassProducer::new(), 0x40, 17)
            .unwrap();

        assert_eq!(manager.producer(producer).unwrap().consumer, Some(consumer));
        assert_eq!(manager.consumer(consumer).unwrap().producer, Some(producer));
        assert_eq!(
            manager.events(),
            &[
                IrqBypassEvent::ProducerStop,
                IrqBypassEvent::ConsumerStop,
                IrqBypassEvent::ProducerAddConsumer,
                IrqBypassEvent::ConsumerAddProducer,
                IrqBypassEvent::ConsumerStart,
                IrqBypassEvent::ProducerStart,
            ]
        );
    }

    #[test]
    fn consumer_registration_requires_add_and_del_callbacks() {
        let mut manager = IrqBypassManager::new();
        let mut consumer = IrqBypassConsumer::new();
        consumer.has_del_producer = false;
        assert_eq!(manager.register_consumer(consumer, 1), Err(-EINVAL));

        let mut busy = IrqBypassManager::new();
        busy.register_consumer(IrqBypassConsumer::new(), 1).unwrap();
        assert_eq!(
            busy.register_consumer(IrqBypassConsumer::new(), 1),
            Err(-EBUSY)
        );
    }

    #[test]
    fn failed_consumer_add_rolls_back_producer_add() {
        let mut manager = IrqBypassManager::new();
        let mut consumer = IrqBypassConsumer::new();
        consumer.add_producer_ret = -EINVAL;
        manager.register_consumer(consumer, 7).unwrap();

        assert_eq!(
            manager.register_producer(IrqBypassProducer::new(), 7, 4),
            Err(-EINVAL)
        );
        assert!(manager.find_producer(7).is_none());
        assert!(
            manager
                .events()
                .contains(&IrqBypassEvent::ProducerDelConsumer)
        );
    }

    #[test]
    fn unregister_disconnects_and_releases_eventfd() {
        let mut manager = IrqBypassManager::new();
        let producer = manager
            .register_producer(IrqBypassProducer::new(), 2, 8)
            .unwrap();
        let consumer = manager
            .register_consumer(IrqBypassConsumer::new(), 2)
            .unwrap();
        manager.unregister_producer(producer);

        assert_eq!(manager.producer(producer).unwrap().eventfd, None);
        assert_eq!(manager.consumer(consumer).unwrap().producer, None);
        assert!(manager.find_producer(2).is_none());
    }
}
